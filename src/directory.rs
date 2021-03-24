/// This file is used for 
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

struct File {
  name: OsString,
  last_modified: String,
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


/// Only public API of directory.rs
pub(crate) fn remove_unversioned(instructions: &Vec<Instruction>, renegadex_location: &String) -> Result<(), Error> {
  let mut versioned_files = Directory::new();
  let renegadex_path = std::path::PathBuf::from(renegadex_location);
  for entry in instructions.iter() {
    let mut path = &mut versioned_files;
    let mut directory_iter = std::path::PathBuf::from(&entry.path)
      .strip_prefix(&renegadex_path)
      .unexpected("Unexpected error occurred")
      .to_path_buf();
    directory_iter.pop();
    for directory in directory_iter.iter() {
      path = path.get_or_create_subdirectory(directory.to_owned());
    }
    //path should be the correct directory now.
    //thus add file to path.files
    if entry.newest_hash.is_some() {
      path.files.push(
        std::path::PathBuf::from(&entry.path)
          .strip_prefix(&renegadex_path)
          .unexpected("Unexpected error occurred")
          .to_path_buf(),
      );
    }
  }
  match std::fs::read_dir(renegadex_location) {
    Ok(_) => {}
    Err(_) => std::fs::create_dir_all(renegadex_location).unexpected("Unexpected error occurred"),
  }
  let files = std::fs::read_dir(renegadex_location).unexpected("");
  for file in files {
    let file = file.unexpected("Unexpected error occurred");
    if file.file_type().unexpected("Unexpected error occurred").is_dir()
    {
      if versioned_files.directory_exists(
        file.path().strip_prefix(&renegadex_path).unexpected("Unexpected error occurred").to_owned(),
      ) {
        read_dir(&file.path(), &versioned_files, &renegadex_path)?;
      } else {
        info!("Remove directory: {:?}", &file.path());
      }
    } else {
      info!("Remove file: {:?}", &file.path());
      //doubt anything
    }
  }
  Ok(())
}

fn read_dir(
  dir: &std::path::Path,
  versioned_files: &Directory,
  renegadex_path: &std::path::PathBuf,
) -> Result<(), Error> {
  let files = std::fs::read_dir(dir).unexpected("Unexpected error occurred");
  for file in files {
    let file = file.unexpected("Unexpected error occurred");
    if file.file_type().unexpected("Unexpected error occurred").is_dir()
    {
      if versioned_files.directory_exists(file.path().strip_prefix(&renegadex_path).unexpected("Unexpected error occurred").to_owned()) {
        read_dir(&file.path(), versioned_files, renegadex_path)?;
      } else {
        info!("Removing directory: {:?}", &file.path());
        std::fs::remove_dir_all(&file.path())?;
      }
    } else {
      if !versioned_files.file_exists(
        file
          .path()
          .strip_prefix(&renegadex_path)
          .unexpected("Unexpected error occurred")
          .to_owned(),
      ) {
        info!("Removing file: {:?}", &file.path());
        std::fs::remove_file(&file.path())?;
      }
      //doubt anything
    }
  }
  Ok(())
}
