use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Directory {
  pub name: OsString,
  pub subdirectories: Vec<Directory>,
  pub files: Vec<PathBuf>,
}