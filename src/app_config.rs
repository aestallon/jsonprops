use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

use clap::{Parser, ValueEnum};

use crate::str_constant;

#[derive(Parser, Debug)]
pub struct Config {
  /// The source JSON file to parse.
  ///
  /// Detailed description here...
  #[arg()]
  source: PathBuf,

  /// The destination .properties file; if not provided, the output will be printed to the standard 
  /// output.
  #[arg()]
  dest: Option<PathBuf>,

  /// Raises the logging level to DEBUG.
  ///
  /// Detailed description here...
  #[arg(short, long)]
  pub debug: bool,

  /// Defines the behaviour for handling lists.
  #[arg(short, long, value_enum, default_value_t = ListHandling::SingleProp)]
  list_handling: ListHandling,

  /// Defines the character sequence for separating keys and values.
  #[arg(short, long, value_enum, default_value_t = EntrySeparator::Equals)]
  entry_separator: EntrySeparator,

  #[arg(long)]
  pub discard_wsp: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum ListHandling {
  SingleProp,
  MultiProp,
}


#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum EntrySeparator {
  Colon,
  Equals,
  Space,
}

#[derive(Debug)]
pub enum ConfigValidationError {
  InvalidPathError(String),
  MissingFileError(String),
}

impl Display for ConfigValidationError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::InvalidPathError(s) => write!(f, "Invalid filepath: {s}"),
      Self::MissingFileError(s) => write!(f, "File does not exist: {s}")
    }
  }
}

impl Error for ConfigValidationError {}

impl Config {
  // This is here only to allow module props tests to create a config:
  #[allow(dead_code)]
  pub fn empty() -> Config {
    Config {
      source: PathBuf::new(),
      dest: None,
      debug: true,
      list_handling: ListHandling::MultiProp,
      entry_separator: EntrySeparator::Equals,
      discard_wsp: false,
    }
  }

  pub fn validate(self) -> Result<Self, ConfigValidationError> {
    let source = &self.source;
    let source_exists = source
      .try_exists()
      .map_err(|_| Self::invalid_path_error(source))?;
    if !source_exists {
      return Err(ConfigValidationError::MissingFileError(Self::path_to_string(source)));
    }

    if let Some(dest) = &self.dest {
      let _ = dest.try_exists().map_err(|_| Self::invalid_path_error(dest))?;
    }

    Ok(self)
  }

  fn invalid_path_error(path: &Path) -> ConfigValidationError {
    ConfigValidationError::InvalidPathError(Self::path_to_string(path))
  }

  fn path_to_string(path: &Path) -> String {
    String::from(path.to_str().unwrap_or(str_constant::EMPTY))
  }

  pub fn source(&self) -> &Path {
    &self.source
  }

  pub fn dest(&self) -> Option<&Path> {
    self.dest.as_deref()
  }

  pub fn list_handling(&self) -> &ListHandling {
    &self.list_handling
  }

  pub fn entry_separator(&self) -> &'static str {
    match self.entry_separator {
      EntrySeparator::Equals => str_constant::EQ,
      EntrySeparator::Colon => str_constant::COLON,
      EntrySeparator::Space => str_constant::SPACE,
    }
  }
}
