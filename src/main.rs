use std::{
	error::Error,
	fmt::Display,
	fs::{self, File},
	io::{self, BufReader},
	path::{self, Path, PathBuf},
	time::Instant,
};

use clap::Parser;
use profile::{Profile, ProfileError};
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

mod profile;

/// Performs archiving on directories using profiles.
#[derive(Debug, Parser)]
#[command(author, version, about, long_about)]
struct Args {
	/// Specifies the profile file
	#[arg(short, long)]
	profile: PathBuf,

	/// Specifies the output file
	#[arg(short, long)]
	file: PathBuf,
}

fn main() {
	let args = Args::parse();

	match archive(args.profile, args.file) {
		Ok(()) => println!("Successfully archived profile."),
		Err(e) => println!("Failed to archive profile: {}.", e),
	}
}

/// Represents an archive-related error.
#[derive(Debug)]
pub enum ArchiveError {
	/// Indicates that the profile could not be loaded.
	FailedToLoad(ProfileError),

	/// Indicates that the metadata for a particular path could not be read.
	FailedToInspectPath(io::Error),

	/// Indicates that the initial archive file could not be created.
	FailedToCreateArchive(io::Error),

	/// Indicates that a particular directory could not be read for its files.
	FailedToReadDirectory(io::Error),

	/// Indicates that a particular file could not be read for its contents.
	FailedToReadFile(io::Error),

	/// Indicates that a specific file could not be copied to the archive.
	FailedToCopyFile(io::Error),

	/// Indicates that a new entry could not be marked in the archive.
	FailedToMarkEntry(zip::result::ZipError),

	/// Indicates that the finished archive file could not be saved.
	FailedToFinishArchive(zip::result::ZipError),

	/// Indicates that the prefix for a particular entry could not be stripped.
	FailedToStripPrefix(path::StripPrefixError),

	/// Indicates that the shared parent path between entries could not be determined.
	FailedToDetermineParentPath,
}

pub type ArchiveResult = Result<(), ArchiveError>;

impl Display for ArchiveError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::FailedToLoad(e) => write!(f, "failed to load profile [{}]", e),
			Self::FailedToInspectPath(e) => write!(f, "failed to inspect path [{}]", e),
			Self::FailedToCreateArchive(e) => write!(f, "failed to create archive file [{}]", e),
			Self::FailedToReadDirectory(e) => write!(f, "failed to read directory [{}]", e),
			Self::FailedToReadFile(e) => write!(f, "failed to read file [{}]", e),
			Self::FailedToCopyFile(e) => write!(f, "failed to copy file to archive [{}]", e),
			Self::FailedToMarkEntry(e) => write!(f, "failed to mark entry in archive [{}]", e),
			Self::FailedToFinishArchive(e) => write!(f, "failed to finish archive [{}]", e),
			Self::FailedToStripPrefix(e) => write!(f, "failed to strip prefix [{}]", e),
			Self::FailedToDetermineParentPath => write!(f, "failed to determine shared parent path"),
		}
	}
}

impl Error for ArchiveError {}

/// Archives the entries described by the specified profile to the specified file.
pub fn archive<T, V>(profile: T, file: V) -> ArchiveResult
where
	T: AsRef<Path>,
	V: AsRef<Path>,
{
	println!("Loading profile from path <{}>...", profile.as_ref().display());

	let profile = Profile::load(profile).map_err(ArchiveError::FailedToLoad)?;

	println!("Creating archive using profile '{}'...", profile.name);

	let start = Instant::now();
	let file = File::create(file).map_err(ArchiveError::FailedToCreateArchive)?;

	// Determine the shared parent path.

	#[rustfmt::skip]
	let parent = profile.dirs.iter()
		.flat_map(|path|
			path.ancestors().find(|ancestor|
				profile.dirs.iter().all(|path|
					path.starts_with(ancestor))))
		.next()
		.ok_or(ArchiveError::FailedToDetermineParentPath)?;

	let mut writer = ZipWriter::new(file);

	// Iterate and archive each directory and its contents.

	println!("Archiving {} directories...", profile.dirs.len());

	for dir in &profile.dirs {
		println!("Archiving directory <{}>...", dir.display());

		if let Err(e) = compress_dir(&mut writer, &profile, &parent, dir) {
			println!("Failed to archive directory: {}.", e);
		}
	}

	// Finish the resulting archive.

	println!("Finishing archive...");

	writer.set_comment(format!("Directory Archiver [{}]", &profile.name));
	writer.finish().map_err(ArchiveError::FailedToFinishArchive)?;

	println!("Created and finished archive in {:#?}.", start.elapsed());

	Ok(())
}

/// Attempts to recursively compress the specified sub-directory to the specified writer.
fn compress_dir<T, V>(writer: &mut ZipWriter<File>, profile: &Profile, parent: T, dir: V) -> ArchiveResult
where
	T: AsRef<Path>,
	V: AsRef<Path>,
{
	if is_ignored(profile, &dir) {
		return Ok(());
	}

	let entries = fs::read_dir(&dir).map_err(ArchiveError::FailedToReadDirectory)?.flatten();
	let path = dir.as_ref().strip_prefix(&parent).map_err(ArchiveError::FailedToStripPrefix)?;

	#[allow(deprecated)]
	writer
		.add_directory_from_path(path, FileOptions::default().compression_method(CompressionMethod::Bzip2).compression_level(Some(9)))
		.map_err(ArchiveError::FailedToMarkEntry)?;

	// Recursively compress each entry.

	for entry in entries {
		let path = entry.path();

		match entry.metadata().map_err(ArchiveError::FailedToInspectPath)? {
			m if m.is_dir() => {
				if let Err(e) = compress_dir(writer, profile, parent.as_ref(), &path) {
					println!("Failed to include sub-directory <{}>: {}.", path.display(), e);
				}
			}
			m if m.is_file() => {
				if let Err(e) = compress_file(writer, profile, parent.as_ref(), &path) {
					println!("Failed to include sub-file <{}>: {}.", path.display(), e);
				}
			}
			_ => {}
		};
	}

	Ok(())
}

/// Attempts to compress the specified sub-file to the specified writer.
fn compress_file<T, V>(writer: &mut ZipWriter<File>, profile: &Profile, parent: T, file: V) -> ArchiveResult
where
	T: AsRef<Path>,
	V: AsRef<Path>,
{
	if is_ignored(profile, &file) {
		return Ok(());
	}

	println!("Compressing file <{}>...", file.as_ref().display());

	let entry = File::open(&file).map_err(ArchiveError::FailedToReadFile)?;
	let path = file.as_ref().strip_prefix(&parent).map_err(ArchiveError::FailedToStripPrefix)?;

	// Compress the entry.

	let mut reader = BufReader::new(entry);

	#[allow(deprecated)]
	writer
		.start_file_from_path(path, FileOptions::default().compression_method(CompressionMethod::Bzip2).compression_level(Some(9)))
		.map_err(ArchiveError::FailedToMarkEntry)?;

	io::copy(&mut reader, writer).map_err(ArchiveError::FailedToCopyFile)?;

	Ok(())
}

/// Determines if the specified path is ignored at all by the specified profile.
fn is_ignored<T>(profile: &Profile, path: T) -> bool
where
	T: AsRef<Path>,
{
	#[rustfmt::skip]
	let ignored = path.as_ref()
		.file_name()
		.and_then(|n| n.to_str())
		.map(|n| profile.ignores.iter().any(|i| i == n))
		.unwrap_or(false);

	ignored
}
