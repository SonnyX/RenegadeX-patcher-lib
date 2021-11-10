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
use crate::structures::{Error, Mirrors, VersionInformation};

pub struct Patcher {
  pub in_progress: Arc<AtomicBool>,
  pub(crate) join_handle: Option<tokio::task::JoinHandle<()>>,
  pub(crate) software_location: String,
  pub(crate) version_url: String
}

impl Patcher {

  pub async fn get_remote_version(&self) -> Result<VersionInformation, Error> {
    VersionInformation::retrieve(&self.version_url).await
  }

  pub async fn start_validation() {

  }

  pub async fn start_patching(&mut self) {
    let join_handle = tokio::task::spawn(async {
      tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

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
}