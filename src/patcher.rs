//Standard library
use std::sync::{Arc};
use std::sync::atomic::AtomicBool;

use crate::functions::flow;
use crate::pausable::BackgroundService;
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
}

async fn pausable_flow(mirrors: Mirrors, software_location: String, instructions_hash: String, success_callback: Box<dyn FnOnce() + Send>, failure_callback: Box<dyn FnOnce(Error) + Send>, progress_callback: Box<dyn Fn(&Progress) + Send>) -> () {
  let result = flow(mirrors, software_location, &instructions_hash, progress_callback).pausable().await;
  if result.is_ok() {
    success_callback();
  } else if let Err(e) = result {
    failure_callback(e);
  }
}

impl Patcher {
  pub async fn start_validation() {

  }

  pub async fn start_patching(&mut self) {
    let mirrors = self.mirrors.clone();
    let software_location = self.software_location.clone();
    let instructions_hash = self.instructions_hash.clone();
    let success_callback = self.success_callback.take().expect("Can only start patching once");
    let failure_callback = self.failure_callback.take().expect("Can only start patching once");
    let progress_callback = self.progress_callback.take().expect("Can only start patching once");

    self.join_handle = Some(tokio::spawn(pausable_flow(mirrors, software_location, instructions_hash, success_callback, failure_callback, progress_callback)));
  }

  pub async fn cancel(mut self) -> Result<(), ()> {
    crate::pausable::FUTURE_CONTEXT.stop()?;
    if let Some(join_handle) = self.join_handle.take() {
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