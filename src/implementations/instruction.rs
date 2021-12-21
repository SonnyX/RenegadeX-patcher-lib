use std::path::Path;

use tokio::fs;

use crate::functions::{delete_file, get_hash, restore_backup};
use crate::structures::{Action, DownloadEntry, Error, Instruction};

impl Instruction {
  pub async fn determine_action(self: Instruction, game_location: String) -> Result<Action, Error> {
    let path = format!("{}{}", &game_location, &self.path);
    let backup_path = format!("{}.bck", &path);
    let mut backup_hash = None;
    let path_exists = fs::metadata(Path::new(&path)).await.is_ok();
    log::info!("Started determine_action regarding: {}", &path);
    let backup_exists = fs::metadata(Path::new(&backup_path)).await.is_ok();
    // Determine wether we have to delete files, update them, or add them.
    if let Some(newest_hash) = self.newest_hash.clone() {
      let mut hash = None;
      // Update or download
      if path_exists {
        hash = Some(get_hash(&path).await?);
        if newest_hash.eq(&hash.clone().unwrap()) {
          // File is already newest file
          if backup_exists {
            delete_file(&backup_path).await;
          }
          log::info!("Done determine_action: {}", &path);
          return Ok(Action::Nothing);
        }
      }
      
      if backup_exists {
        backup_hash = Some(get_hash(&backup_path).await?);
        if backup_hash.clone().map(|backup_hash| newest_hash.eq(&backup_hash)).unwrap() {
          // Restore backup file
          restore_backup(&path).await;
          log::info!("Done determine_action: {}", &path);
          return Ok(Action::Nothing);
        }
      }
      
      // File is not up to date
      if let Some(previous_hash) = self.previous_hash.clone() {
        if path_exists && previous_hash.eq(&hash.clone().unwrap()) {
          // Download delta
          log::info!("Done determine_action: {}", &path);
          return Ok(Action::Download(DownloadEntry {
            mirror_path: format!("delta/{}_from_{}", &newest_hash, &previous_hash),
            download_path: self.delta_vcdiff_hash.clone().ok_or(Error::None(format!("Expected instruction to have full_vcdiff_hash, however there was None: {:#?}", self)))?,
            download_size: self.delta_vcdiff_size,
            download_hash: self.delta_vcdiff_hash.clone().ok_or(Error::None(format!("Expected instruction to have full_vcdiff_hash, however there was None: {:#?}", self)))?,
            target_path: path,
            target_hash: newest_hash,
          }));
        // Check if there's a backup file, and restore it if it matches previous_hash
        } else if backup_exists && previous_hash.eq(&backup_hash.clone().unwrap()) {
          // Restore backup file
          restore_backup(&path).await;
          log::info!("Done determine_action: {}", &path);
          return Ok(Action::Download(DownloadEntry {
            mirror_path: format!("delta/{}_from_{}", &newest_hash, &previous_hash),
            download_path: self.delta_vcdiff_hash.clone().ok_or(Error::None(format!("Expected instruction to have full_vcdiff_hash, however there was None: {:#?}", self)))?,
            download_size: self.delta_vcdiff_size,
            download_hash: self.delta_vcdiff_hash.clone().ok_or(Error::None(format!("Expected instruction to have full_vcdiff_hash, however there was None: {:#?}", self)))?,
            target_path: path,
            target_hash: newest_hash,
          }));
        }
      }
      
      // Download full
      log::info!("Done determine_action: {}", &path);
      return Ok(Action::Download(DownloadEntry {
        mirror_path: format!("full/{}", &newest_hash),
        download_path: self.full_vcdiff_hash.clone().ok_or(Error::None(format!("Expected instruction to have full_vcdiff_hash, however there was None: {:#?}", self)))?,
        download_size: self.full_vcdiff_size,
        download_hash: self.full_vcdiff_hash.clone().ok_or(Error::None(format!("Expected instruction to have full_vcdiff_hash, however there was None: {:#?}", self)))?,
        target_path: path,
        target_hash: newest_hash,
      }));
    } else {
      // Delete file
      if backup_exists {
        delete_file(&backup_path).await;
      }
      if path_exists {
        delete_file(&path).await;
      }
    }
    log::info!("Done determine_action: {}", &path);
    Ok(Action::Nothing)
  }
}
