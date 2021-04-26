use crate::traits::ExpectUnwrap;
use std::ffi::OsString;
use std::path::PathBuf;
use crate::instructions::Instruction;
use log::*;
use crate::traits::Error;


#[derive(Debug)]
struct Directory {
  name: OsString,
  subdirectories: Vec<Directory>,
  files: Vec<PathBuf>,
}


impl Directory {
  fn new() -> Self {
    Self {
      name: "".into(),
      subdirectories: Vec::new(),
      files: Vec::new(),
    }
  }

  fn with_name(name: OsString) -> Self {
    Self {
      name,
      subdirectories: Vec::new(),
      files: Vec::new(),
    }
  }

  /// Get or Create a subdirectory
  fn get_or_create_subdirectory(&mut self, name: OsString) -> &mut Directory {
    for index in 0..self.subdirectories.len() {
      if self.subdirectories[index].name == name {
        return &mut self.subdirectories[index];
      }
    }
    self.subdirectories.push(Directory::with_name(name));
    return self.subdirectories.last_mut().unexpected("Unexpected error occurred");
  }

  /// 
  fn get_subdirectory(&self, name: OsString) -> Option<&Directory> {
    for index in 0..self.subdirectories.len() {
      if self.subdirectories[index].name == name {
        return Some(&self.subdirectories[index]);
      }
    }
    return None;
  }

  fn directory_exists(&self, path: PathBuf) -> bool {
    //split up path into an iter and push it to temporary path's, if it's all done then we're good
    let mut temp = self;
    for directory in path.iter() {
      temp = match temp.get_subdirectory(directory.to_owned()) {
        Some(subdir) => subdir,
        None => {
          return false;
        }
      };
    }
    return true;
  }

  fn file_exists(&self, file: PathBuf) -> bool {
    //split up path into an iter and push it to temporary path's, if it's all done then we're good

    // I'm actually confused, why do we return if the file is InstallInfo.xml? the idea is that we shouldn't delete this file, but why here
    if file.file_name().unexpected("Unexpected error occurred") == "InstallInfo.xml" {
      return true;
    }
    let mut temp = self;
    let mut dir = file.clone();
    dir.pop();
    for directory in dir.iter() {
      temp = match temp.get_subdirectory(directory.to_owned()) {
        Some(subdir) => subdir,
        None => {
          return false;
        }
      };
    }
    return temp.files.contains(&file);
  }
}