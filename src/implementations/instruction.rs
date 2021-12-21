use std::path::Path;

use crate::functions::{delete_file, get_hash, restore_backup};
use crate::structures::{Action, DownloadEntry, Error, Instruction};

impl Instruction {
  pub async fn determine_action(self: Instruction) -> Result<Action, Error> {
    let backup_path = format!("{}.bck", &self.path);
    let mut backup_hash = None;
    let path_exists = Path::new(&self.path).exists();
    let backup_exists = Path::new(&backup_path).exists();
    // Determine wether we have to delete files, update them, or add them.
    if let Some(newest_hash) = self.newest_hash.clone() {
      let mut hash = None;
      // Update or download
      if path_exists {
        hash = Some(get_hash(&self.path).await?);
        if newest_hash.eq(&hash.clone().unwrap()) {
          // File is already newest file
          if backup_exists {
            delete_file(&backup_path).await;
          }
          return Ok(Action::Nothing);
        }
      }
      
      if backup_exists {
        backup_hash = Some(get_hash(&backup_path).await?);
        if backup_hash.clone().map(|backup_hash| newest_hash.eq(&backup_hash)).unwrap() {
          // Restore backup file
          restore_backup(&self.path).await;
          return Ok(Action::Nothing);
        }
      }
      
      // File is not up to date
      if let Some(previous_hash) = self.previous_hash.clone() {
        if path_exists && previous_hash.eq(&hash.clone().unwrap()) {
          // Download delta
          return Ok(Action::Download(DownloadEntry {
            mirror_path: format!("delta/{}_from_{}", &newest_hash, &previous_hash),
            download_path: self.delta_vcdiff_hash.clone().ok_or(Error::None(format!("Expected instruction to have full_vcdiff_hash, however there was None: {:#?}", self)))?,
            download_size: self.delta_vcdiff_size,
            download_hash: self.delta_vcdiff_hash.clone().ok_or(Error::None(format!("Expected instruction to have full_vcdiff_hash, however there was None: {:#?}", self)))?,
            target_path: self.path,
            target_hash: newest_hash,
          }));
        // Check if there's a backup file, and restore it if it matches previous_hash
        } else if backup_exists && previous_hash.eq(&backup_hash.clone().unwrap()) {
          // Restore backup file
          restore_backup(&self.path).await;

          return Ok(Action::Download(DownloadEntry {
            mirror_path: format!("delta/{}_from_{}", &newest_hash, &previous_hash),
            download_path: self.delta_vcdiff_hash.clone().ok_or(Error::None(format!("Expected instruction to have full_vcdiff_hash, however there was None: {:#?}", self)))?,
            download_size: self.delta_vcdiff_size,
            download_hash: self.delta_vcdiff_hash.clone().ok_or(Error::None(format!("Expected instruction to have full_vcdiff_hash, however there was None: {:#?}", self)))?,
            target_path: self.path,
            target_hash: newest_hash,
          }));
        }
      }
      
      // Download full
      return Ok(Action::Download(DownloadEntry {
        mirror_path: format!("full/{}", &newest_hash),
        download_path: self.full_vcdiff_hash.clone().ok_or(Error::None(format!("Expected instruction to have full_vcdiff_hash, however there was None: {:#?}", self)))?,
        download_size: self.full_vcdiff_size,
        download_hash: self.full_vcdiff_hash.clone().ok_or(Error::None(format!("Expected instruction to have full_vcdiff_hash, however there was None: {:#?}", self)))?,
        target_path: self.path,
        target_hash: newest_hash,
      }));
    } else {
      // Delete file
      if backup_exists {
        delete_file(&backup_path).await;
      }
      if path_exists {
        delete_file(&self.path).await;
      }
    }
    Ok(Action::Nothing)
  }
}
