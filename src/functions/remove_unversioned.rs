use crate::{structures::{Directory, Error, Instruction}, functions::read_dir, Progress};
use tracing::info;
use std::path::PathBuf;

/// This function converts the instructions array to a Directory structure
fn instructions_to_directory_info(instructions: &Vec<Instruction>, renegadex_path: &PathBuf) -> Result<Directory, Error> {
  let mut versioned_files = Directory::new();
  // build up directory structure based on instructions.json
  for entry in instructions.iter() {
    let mut path = &mut versioned_files;
    let mut directory_iter = PathBuf::from(&entry.path).strip_prefix(&renegadex_path)?.to_path_buf();
    directory_iter.pop();
    for directory in directory_iter.iter() {
      path = path.get_or_create_subdirectory(directory.to_owned())?;
    }
    //path should be the correct directory now.
    //thus add file to path.files
    if entry.newest_hash.is_some() {
      path.files.push(
        PathBuf::from(&entry.path)
          .strip_prefix(&renegadex_path)?
          .to_path_buf(),
      );
    }
  }
  Ok(versioned_files)
}


pub(crate) async fn remove_unversioned(game_location: String, instructions: Vec<Instruction>, progress: Progress, progress_callback: Box<dyn Fn(&Progress) + Send>) -> Result<(), Error> {
    let game_path = std::path::PathBuf::from(game_location.clone());
    progress.set_current_action("Removing unknown files!".to_string())?;
    progress_callback(&progress);

    let versioned_files = instructions_to_directory_info(&instructions, &game_path)?;

    // Create the game directory if it doesn't exist already
    match std::fs::read_dir(game_location.clone()) {
      Ok(_) => {}
      Err(_) => std::fs::create_dir_all(game_location.clone())?,
    };

    // Iterate through the files and remove the unversioned
    let files = std::fs::read_dir(game_location.clone())?;
    for file in files {
      let file = file?;
      if file.file_type()?.is_dir() {
        if versioned_files.directory_exists(file.path().strip_prefix(&game_path)?.to_owned()) {
          read_dir(&file.path(), &versioned_files, &game_path)?;
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