//Standard library
use std::sync::{Arc};
use std::sync::atomic::AtomicBool;


use crate::functions::flow;
use crate::pausable::BackgroundService;
use crate::pausable::PausableTrait;
use crate::structures::{Error, Mirrors};

pub struct Patcher {
  pub in_progress: Arc<AtomicBool>,
  pub(crate) join_handle: Option<tokio::task::JoinHandle<()>>,
  pub(crate) software_location: String,
  pub(crate) mirrors: Mirrors,
  pub(crate) instructions_hash: String,
}

impl Patcher {
  pub async fn start_validation() {

  }

  pub async fn start_patching(&mut self) {
    let mut mirrors = self.mirrors.clone();
    let software_location = self.software_location.clone();
    let instructions_hash = self.instructions_hash.clone();

    let join_handle = tokio::task::spawn(async move {
      let result = flow(mirrors, software_location, instructions_hash).pausable().await;
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