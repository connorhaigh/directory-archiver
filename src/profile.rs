use core::fmt;
use std::{
	error::Error,
	fmt::Display,
	fs, io,
	path::{Path, PathBuf},
};

use serde::Deserialize;

/// Represents a profile.
#[derive(Debug, Deserialize)]
pub struct Profile {
	/// The display name.
	pub name: String,

	/// The paths of directories that will be included.
	pub directories: Vec<PathBuf>,

	/// The wildcard patterns for directory names and file names that should be ignored.
	pub ignores: Vec<String>,
}

/// Represents a profile-related error.
#[derive(Debug)]
pub enum ProfileError {
	/// Indicates that a profile could not be read.
	FailedToRead(io::Error),

	/// Indicates that the JSON representing a profile could not be parsed.
	FailedToDeserialise(serde_json::Error),
}

pub type ProfileResult = Result<Profile, ProfileError>;

impl Profile {
	pub fn load<T>(path: T) -> ProfileResult
	where
		T: AsRef<Path>,
	{
		let json = fs::read_to_string(&path).map_err(ProfileError::FailedToRead)?;
		let profile = serde_json::from_str(&json).map_err(ProfileError::FailedToDeserialise)?;

		Ok(profile)
	}
}

impl Error for ProfileError {}

impl Display for ProfileError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::FailedToRead(e) => write!(f, "failed to read file [{}]", e),
			Self::FailedToDeserialise(e) => write!(f, "failed to deserialise value [{}]", e),
		}
	}
}
