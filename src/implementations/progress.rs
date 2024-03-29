use async_trait::async_trait;
use tracing::info;

use crate::structures::{Error, Progress};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[async_trait]
impl download_async::Progress for Progress {
    async fn set_file_size(&mut self, _size: usize) -> () {
        // We already know the file-size beforehand
    }

    async fn add_to_progress(&mut self, amount: usize) -> () {
      self.downloaded_bytes.0.fetch_add(amount as u64, Ordering::Relaxed);

    }

    async fn remove_from_progress(&mut self, bytes: usize) -> () {
      self.downloaded_bytes.0.fetch_sub(bytes as u64, Ordering::Relaxed);
    }
}

impl Progress {
    pub fn new() -> Self {
        Self {
            current_action: Arc::new(Mutex::new(format!(""))),
            processed_instructions: Arc::new((AtomicU64::new(0), AtomicU64::new(0))),
            downloaded_files: Arc::new((AtomicU64::new(0), AtomicU64::new(0))),
            downloaded_bytes: Arc::new((AtomicU64::new(0), AtomicU64::new(0))),
            patched_files: Arc::new((AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0))),
            patched_bytes: Arc::new((AtomicU64::new(0), AtomicU64::new(0))),
        }
    }

    pub fn get_current_action(&self) -> Result<String, Error> {
        Ok((*self.current_action.lock()?).clone())
    }

    pub(crate) fn set_current_action(&self, value: String) -> Result<(), Error> {
        info!("Current action: {}", value);
        *self.current_action.lock()? = value;
        Ok(())
    }

    pub(crate) fn set_instructions_amount(&self, value: u64) {
        info!("Amount of instructions: {}", value);
        self.processed_instructions.1.store(value, Ordering::Relaxed);
    }

    pub(crate) fn increment_processed_instructions(&self) {
        self.processed_instructions.0.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn add_download(&self, value: u64) {
        self.downloaded_files.1.fetch_add(1, Ordering::Relaxed);
        self.downloaded_bytes.1.fetch_add(value, Ordering::Relaxed);
    }

    pub(crate) fn increment_completed_downloads(&self) {
        self.downloaded_files.0.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn add_to_be_patched(&self) {
        self.patched_files.2.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn add_ready_to_patch(&self) {
        self.patched_files.1.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn increment_completed_patches(&self) {
        self.patched_files.0.fetch_add(1, Ordering::Relaxed);
    }
}