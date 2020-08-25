use crate::traits::ExpectUnwrap;
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) struct Directory {
  pub name: OsString,
  pub subdirectories: Vec<Directory>,
  pub files: Vec<PathBuf>,
}

impl Directory {
  pub fn get_or_create_subdirectory(&mut self, name: OsString) -> &mut Directory {
    for index in 0..self.subdirectories.len() {
      if self.subdirectories[index].name == name {
        return &mut self.subdirectories[index];
      }
    }
    self.subdirectories.push(
      Directory {
        name: name,
        subdirectories: Vec::new(),
        files: Vec::new(), 
      }
    );
    return self.subdirectories.last_mut().unexpected(concat!(module_path!(),":",file!(),":",line!()));
  }

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
        },
      };
    }
    return true;
  }

  pub fn file_exists(&self, file: PathBuf) -> bool {
    //split up path into an iter and push it to temporary path's, if it's all done then we're good
    if file.file_name().unexpected(concat!(module_path!(),":",file!(),":",line!())) == "InstallInfo.xml" {
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
        },
      };
    }
    return temp.files.contains(&file);
  }

}