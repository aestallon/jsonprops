use std::env::Args;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

use crate::STR_EMPTY;

#[derive(Debug)]
pub enum AppConfig {
  HelpConfig
}

#[derive(Debug)]
pub struct Config {
  source: PathBuf,
  dest: PathBuf,
  pub debug: bool,
  list_handling: ListHandling,
}

#[derive(Debug)]
pub enum ListHandling {
  SingleProp,
  MultiProp,
}

#[derive(Debug)]
pub enum ConfigCreationError {
  MissingArgumentError,
  InvalidPathError(String),
  MissingFileError(String),
}

impl Display for ConfigCreationError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::MissingArgumentError => write!(f, ""),
      Self::InvalidPathError(s) => write!(f, "Invalid filepath: {s}"),
      Self::MissingFileError(s) => write!(f, "File does not exist: {s}")
    }
  }
}

impl Error for ConfigCreationError {}


impl Config {
  pub fn from_args(args: Args) -> Result<Self, ConfigCreationError> {
    let mut paths: Vec<PathBuf> = args
      .skip(1)
      .map(|it| PathBuf::from(it))
      .collect();
    if paths.len() < 2 {
      return Err(ConfigCreationError::MissingArgumentError);
    }

    let source = paths.remove(0);
    let source_exists = source
      .try_exists()
      .map_err(|_| Self::invalid_path_error(&source))?;
    if !source_exists {
      return Err(ConfigCreationError::MissingFileError(Self::path_to_string(&source)));
    }

    let dest = paths.remove(0);
    let _ = dest.try_exists().map_err(|_| Self::invalid_path_error(&dest))?;

    Ok(Config {
      source,
      dest,
      debug: false,
      list_handling: ListHandling::SingleProp,
    })
  }

  pub fn empty() -> Config {
    Config {
      source: PathBuf::new(),
      dest: PathBuf::new(),
      debug: true,
      list_handling: ListHandling::MultiProp,
    }
  }

  fn invalid_path_error(path: &Path) -> ConfigCreationError {
    ConfigCreationError::InvalidPathError(Self::path_to_string(path))
  }

  fn path_to_string(path: &Path) -> String {
    String::from(path.to_str().unwrap_or_else(|| STR_EMPTY))
  }

  pub fn source(&self) -> &Path {
    &self.source
  }

  pub fn dest(&self) -> &Path {
    &self.dest
  }

  pub fn list_handling(&self) -> &ListHandling {
    &self.list_handling
  }
}
