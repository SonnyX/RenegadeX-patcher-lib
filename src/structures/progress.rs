use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

pub struct Progress {
  pub(crate) current_action: Arc<Mutex<String>>,
  pub(crate) verified_files: (AtomicU64, AtomicU64),
  pub(crate) downloaded_files: (AtomicU64, AtomicU64),
  pub(crate) downloaded_bytes: (AtomicU64, AtomicU64),
  pub(crate) patched_files: (AtomicU64, AtomicU64),
  pub(crate) patched_bytes: (AtomicU64, AtomicU64),
}