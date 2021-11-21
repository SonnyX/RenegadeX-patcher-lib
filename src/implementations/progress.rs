use log::info;

use crate::structures::{Error, Progress};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

impl Progress {
    pub fn new() -> Self {
        Self {
            current_action: Arc::new(Mutex::new(format!(""))),
            processed_instructions: Arc::new((AtomicU64::new(0), AtomicU64::new(0))),
            downloaded_files: Arc::new((AtomicU64::new(0), AtomicU64::new(0))),
            downloaded_bytes: Arc::new((AtomicU64::new(0), AtomicU64::new(0))),
            patched_files: Arc::new((AtomicU64::new(0), AtomicU64::new(0))),
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

    pub(crate) async fn call_every(&self, timespan: Duration) -> Result<(), Error> {
        Ok(())
    }
}