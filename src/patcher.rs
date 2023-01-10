//Standard library
use std::sync::{Arc};
use std::sync::atomic::AtomicBool;

use crate::functions::{flow, remove_unversioned};
use crate::pausable::{BackgroundService, FutureContext};
use crate::pausable::PausableTrait;
use crate::structures::{Error, Mirrors, Progress};

pub struct Patcher {
  pub in_progress: Arc<AtomicBool>,
  pub(crate) join_handle: Option<tokio::task::JoinHandle<()>>,
  pub(crate) software_location: String,
  pub(crate) mirrors: Mirrors,
  pub(crate) instructions_hash: String,
  pub(crate) success_callback: Option<Box<dyn FnOnce() + Send>>,
  pub(crate) failure_callback: Option<Box<dyn FnOnce(Error) + Send>>,
  pub(crate) progress_callback: Option<Box<dyn Fn(&Progress) + Send>>,
  pub(crate) context: Arc<FutureContext>
}

impl Patcher {
  pub async fn remove_unversioned(&mut self) {
    let mirrors = self.mirrors.clone();
    let software_location = self.software_location.clone();
    let instructions_hash = self.instructions_hash.clone();
    let success_callback = self.success_callback.take().expect("Can only start patching once");
    let failure_callback = self.failure_callback.take().expect("Can only start patching once");
    let progress_callback = self.progress_callback.take().expect("Can only start patching once");
    let context = self.context.clone();

    self.join_handle = Some(tokio::task::spawn(async move {
      let result = remove_unversioned(mirrors, software_location, &instructions_hash, progress_callback, context.clone()).pausable(context).await;
      if result.is_ok() {
        tracing::info!("Calling success_callback");
        success_callback();
      } else if let Err(e) = result {
        tracing::info!("Calling failure_callback");
        failure_callback(e);
      }
    }));
  }

  pub async fn start_patching(&mut self) {
    let mirrors = self.mirrors.clone();
    let software_location = self.software_location.clone();
    let instructions_hash = self.instructions_hash.clone();
    let success_callback = self.success_callback.take().expect("Can only start patching once");
    let failure_callback = self.failure_callback.take().expect("Can only start patching once");
    let progress_callback = self.progress_callback.take().expect("Can only start patching once");
    let context = self.context.clone();

    self.join_handle = Some(tokio::task::spawn(async move {
      let result = flow(mirrors, software_location, &instructions_hash, progress_callback, context.clone()).pausable(context).await;
      if result.is_ok() {
        tracing::info!("Calling success_callback");
        success_callback();
      } else if let Err(e) = result {
        tracing::info!("Calling failure_callback");
        failure_callback(e);
      }
    }));
  }

  pub async fn get_handle(mut self) -> Option<tokio::task::JoinHandle<()>> {
    self.join_handle.take()
  } 

  pub async fn cancel(mut self) -> Result<(), ()> {
    self.context.stop()?;
    if let Some(join_handle) = self.join_handle.take() {
      let _ = join_handle.await;
    }
    Ok(())
  }

  pub fn pause(&self) -> Result<(), ()> {
    self.context.pause()
  }

  pub fn resume(&self) -> Result<(), ()> {
    self.context.resume()
  }
}