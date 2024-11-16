use core::fmt;
use std::{
	error::Error,
	fmt::Display,
	fs, io,
	path::{Path, PathBuf},
};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Profile {
	pub name: String,

	pub dirs: Vec<PathBuf>,
	pub ignores: Vec<String>,
}

#[derive(Debug)]
pub enum ProfileError {
	FailedToRead(io::Error),
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
