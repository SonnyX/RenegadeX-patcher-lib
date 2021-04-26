use crate::structures::{Directory, Error};
use log::info;

pub fn read_dir(
    dir: &std::path::Path,
    versioned_files: &Directory,
    renegadex_path: &std::path::PathBuf,
  ) -> Result<(), Error> {
    let files = std::fs::read_dir(dir)?;
    for file in files {
      let file = file?;
      if file.file_type()?.is_dir()
      {
        if versioned_files.directory_exists(file.path().strip_prefix(&renegadex_path)?.to_owned()) {
          read_dir(&file.path(), versioned_files, renegadex_path)?;
        } else {
          info!("Removing directory: {:?}", &file.path());
          std::fs::remove_dir_all(&file.path())?;
        }
      } else {
        if !versioned_files.file_exists(file.path().strip_prefix(&renegadex_path)?.to_owned())? {
          info!("Removing file: {:?}", &file.path());
          std::fs::remove_file(&file.path())?;
        }
        //doubt anything
      }
    }
    Ok(())
  }