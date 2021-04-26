use std::ffi::OsString;
use std::path::PathBuf;
use crate::structures::{Directory, Error};

impl Directory {
    pub fn new() -> Self {
      Self {
        name: "".into(),
        subdirectories: Vec::new(),
        files: Vec::new(),
      }
    }
  
    pub fn with_name(name: OsString) -> Self {
      Self {
        name,
        subdirectories: Vec::new(),
        files: Vec::new(),
      }
    }
  
    /// Get or Create a subdirectory
    pub fn get_or_create_subdirectory(&mut self, name: OsString) -> Result<&mut Directory, Error> {
      for index in 0..self.subdirectories.len() {
        if self.subdirectories[index].name == name {
          return Ok(&mut self.subdirectories[index]);
        }
      }
      self.subdirectories.push(Directory::with_name(name));
      return self.subdirectories.last_mut().ok_or_else(|| Error::None(format!("Couldnt get a mutable borrow of the last entry of subdirectories")));
    }
  
    /// 
    pub fn get_subdirectory(&self, name: OsString) -> Option<&Directory> {
      for index in 0..self.subdirectories.len() {
        if self.subdirectories[index].name == name {
          return Some(&self.subdirectories[index]);
        }
      }
      return None;
    }
  
    pub fn directory_exists(&self, path: PathBuf) -> bool {
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
  
    pub fn file_exists(&self, file: PathBuf) -> Result<bool, Error> {
      //split up path into an iter and push it to temporary path's, if it's all done then we're good
  
      // I'm actually confused, why do we return if the file is InstallInfo.xml? the idea is that we shouldn't delete this file, but why here
      if file.file_name().ok_or_else(|| Error::None(format!("FileName of {:?} is None", file)))? == "InstallInfo.xml" {
        return Ok(true);
      }
      let mut temp = self;
      let mut dir = file.clone();
      dir.pop();
      for directory in dir.iter() {
        temp = match temp.get_subdirectory(directory.to_owned()) {
          Some(subdir) => subdir,
          None => {
            return Ok(false);
          }
        };
      }
      return Ok(temp.files.contains(&file));
    }
  }