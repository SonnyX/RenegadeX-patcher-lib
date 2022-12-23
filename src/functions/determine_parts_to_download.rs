use std::{io::{SeekFrom, Write, Seek, Read}, fs::OpenOptions};

use crate::structures::{FilePart};
use crate::{structures::{Error}, functions::get_hash};

pub fn determine_parts_to_download(file_location: &str, file_hash: &str, size: u64) -> Result<(String, Vec<FilePart>), Error> {
  const PART_SIZE : u64 = 2u64.pow(20); //1.048.576 == 1 MB aprox
  let mut f = OpenOptions::new().read(true).write(true).create(true).open(&file_location)?;
  //set the size of the file, add a byte for each part to the end of the file as a means of tracking progress.
  let parts_amount : u64 = size / PART_SIZE + if size % PART_SIZE > 0 {1} else {0};
  let file_size : u64 = size + parts_amount;
  tracing::info!("Getting metadata of {}", &file_location);
  let file_metadata = f.metadata()?;
  if (file_metadata.len()) != file_size {
    if file_metadata.len() == size {
      //If hash is correct, return.
      //Otherwise download again.
      tracing::info!("Getting hash of {}", &file_location);
      let hash = get_hash(&file_location)?;
      if hash == file_hash {
        return Ok((file_location.to_owned(), vec!()));
      }
    }
    tracing::info!("Setting size of {}", &file_location);
    f.set_len(file_size as u64)?;
    f.flush()?;
  }
  //We have set up the file
  tracing::info!("Seeking to location of {}", &file_location);
  f.seek(SeekFrom::Start(size as u64))?;
  let mut completed_parts = vec![0; parts_amount as usize];
  f.read_exact(&mut completed_parts)?;
  f.flush()?;
  
  let download_parts : Vec<FilePart> = completed_parts.iter().enumerate().filter(|(i, part)| part == &&0_u8).map(|(i,_)| FilePart::new(file_location.to_owned(), size + (i as u64), ( i as u64 ) * PART_SIZE, ( ( (i + 1) as u64) * PART_SIZE).min(size))).collect();
  return Ok((file_location.to_owned(), download_parts));
}