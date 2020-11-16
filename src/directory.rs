use crate::traits::ExpectUnwrap;
use std::ffi::OsString;
use std::path::PathBuf;
use crate::instructions::Instruction;
use log::*;
use crate::traits::Error;

#[derive(Debug)]
pub(crate) struct Directory {
  pub name: OsString,
  pub subdirectories: Vec<Directory>,
  pub files: Vec<PathBuf>,
}

pub(crate) struct File {
  name: OsString,
  last_modified: String,
}

impl Directory {
  pub fn get_or_create_subdirectory(&mut self, name: OsString) -> &mut Directory {
    for index in 0..self.subdirectories.len() {
      if self.subdirectories[index].name == name {
        return &mut self.subdirectories[index];
      }
    }
    self.subdirectories.push(Directory {
      name,
      subdirectories: Vec::new(),
      files: Vec::new(),
    });
    return self.subdirectories.last_mut().unexpected(concat!(
      module_path!(),
      ":",
      file!(),
      ":",
      line!()
    ));
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
        }
      };
    }
    return true;
  }

  pub fn file_exists(&self, file: PathBuf) -> bool {
    //split up path into an iter and push it to temporary path's, if it's all done then we're good
    if file
      .file_name()
      .unexpected(concat!(module_path!(), ":", file!(), ":", line!()))
      == "InstallInfo.xml"
    {
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

pub(crate) fn remove_unversioned(instructions: &Vec<Instruction>, renegadex_location: &String) -> Result<(), Error> {
  let mut versioned_files = Directory {
    name: "".into(),
    subdirectories: Vec::new(),
    files: Vec::new(),
  };
  let renegadex_path = std::path::PathBuf::from(renegadex_location);
  for entry in instructions.iter() {
    let mut path = &mut versioned_files;
    let mut directory_iter = std::path::PathBuf::from(&entry.path)
      .strip_prefix(&renegadex_path)
      .unexpected(concat!(module_path!(), ":", file!(), ":", line!()))
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
          .unexpected(concat!(module_path!(), ":", file!(), ":", line!()))
          .to_path_buf(),
      );
    }
  }
  match std::fs::read_dir(renegadex_location) {
    Ok(_) => {}
    Err(_) => std::fs::create_dir_all(renegadex_location).unexpected(concat!(
      module_path!(),
      ":",
      file!(),
      ":",
      line!()
    )),
  }
  let files = std::fs::read_dir(renegadex_location).unexpected(concat!(
    module_path!(),
    ":",
    file!(),
    ":",
    line!()
  ));
  for file in files {
    let file = file.unexpected(concat!(module_path!(), ":", file!(), ":", line!()));
    if file
      .file_type()
      .unexpected(concat!(module_path!(), ":", file!(), ":", line!()))
      .is_dir()
    {
      if versioned_files.directory_exists(
        file
          .path()
          .strip_prefix(&renegadex_path)
          .unexpected(concat!(module_path!(), ":", file!(), ":", line!()))
          .to_owned(),
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
  let files =
    std::fs::read_dir(dir).unexpected(concat!(module_path!(), ":", file!(), ":", line!()));
  for file in files {
    let file = file.unexpected(concat!(module_path!(), ":", file!(), ":", line!()));
    if file
      .file_type()
      .unexpected(concat!(module_path!(), ":", file!(), ":", line!()))
      .is_dir()
    {
      if versioned_files.directory_exists(
        file
          .path()
          .strip_prefix(&renegadex_path)
          .unexpected(concat!(module_path!(), ":", file!(), ":", line!()))
          .to_owned(),
      ) {
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
          .unexpected(concat!(module_path!(), ":", file!(), ":", line!()))
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
