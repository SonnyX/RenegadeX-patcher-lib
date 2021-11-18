use log::info;

use crate::structures::{Error, Progress};
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

impl Progress {
    pub fn new() -> Self {
        Self {
            current_action: Arc::new(Mutex::new(format!(""))),
            verified_files: (AtomicU64::new(0), AtomicU64::new(0)),
            downloaded_files: (AtomicU64::new(0), AtomicU64::new(0)),
            downloaded_bytes: (AtomicU64::new(0), AtomicU64::new(0)),
            patched_files: (AtomicU64::new(0), AtomicU64::new(0)),
            patched_bytes: (AtomicU64::new(0), AtomicU64::new(0)),
        }
    }

    pub fn get_current_action(&self) -> Result<String, Error> {
        Ok((*self.current_action.lock()?).clone())
    }

    pub(crate) fn set_current_action(&self, value: String) -> Result<(), Error> {
        info!("{}", value);
        *self.current_action.lock()? = value;
        Ok(())
    }
}