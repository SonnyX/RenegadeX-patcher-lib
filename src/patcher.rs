//Standard library
use std::collections::BTreeMap;
use std::fs::{OpenOptions,DirBuilder};
use std::io::{Read, Write, Seek, SeekFrom};
use std::iter::FromIterator;
use std::ops::Deref;
use std::panic;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;

//External crates
use rayon::prelude::*;
use ini::Ini;
use log::*;
use download_async::Body;
use futures::task::AtomicWaker;
use futures::future::join_all;

use crate::pausable::BackgroundService;
use crate::pausable::PausableTrait;

pub struct Patcher {
  pub logs: String,
  pub in_progress: Arc<AtomicBool>,
  pub(crate) join_handle: Option<tokio::task::JoinHandle<()>>,
  pub(crate) game_location: String,
  pub(crate) version_url: String
}

impl Patcher {

  pub async fn get_remote_version() {

  }

  pub async fn start_validation() {

  }

  pub async fn start_patching(&mut self) {
    //self.in_progress.swap(val, order);
    let join_handle = tokio::task::spawn(async {
      tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
      /*
      // Download release.json
      if self.version_and_mirror_info.is_empty() {
        self.version_and_mirror_info = download_single_file();
      }

      // Download instructions.json, however only if it hasn't been downloaded yet
      let instructions : Vec<Instruction> = download_instructions().await;

      // Sort instructions.json to be in groups.
      let instructionGroups : Vec<InstructionGroup> = instructions.sort();

      join_all(instructionGroups).pausable().await;
      // For each group:
      //   - check whether one of the files has a file matching with the new hash
      //   - otherwise with the old hash.
      //   - If no new hash exists:
      //     - Download delta or full file
      //     - Patch an old file
      //   - copy over the rest of the files

      // 
      */
    }.pausable());
  }

  pub async fn cancel(mut self) -> Result<(), ()> {
    crate::pausable::FUTURE_CONTEXT.stop()?;
    if let Some(mut join_handle) = self.join_handle.take() {
      let _ = join_handle.await;
    }
    Ok(())
  }

  pub fn pause(&self) -> Result<(), ()> {
    crate::pausable::FUTURE_CONTEXT.pause()
  }

  pub fn resume(&self) -> Result<(), ()> {
    crate::pausable::FUTURE_CONTEXT.resume()
  }

  pub fn get_logs(&self) -> String {
    "".to_string()
  }
}



/*
pub async fn start() {

  async {
    // Download release.json
    if self.version_and_mirror_info.is_empty() {
      self.version_and_mirror_info = download_single_file();
    }

    // Download instructions.json, however only if it hasn't been downloaded yet
    let instructions : Vec<Instruction> = download_instructions().await;

    // Sort instructions.json to be in groups.
    let instructions : Vec<InstructionGroup> = instructions.sort();


    // For each group:
    //   - check whether one of the files has a file matching with the new hash
    //   - otherwise with the old hash.
    //   - If no new hash exists:
    //     - Download delta or full file
    //     - Patch an old file
    //   - copy over the rest of the files

    // 
  }.pausable().await

}
*/