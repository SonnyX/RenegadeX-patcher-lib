use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Progress {
  pub(crate) current_action: Arc<Mutex<String>>,
  pub(crate) verified_files: Arc<(AtomicU64, AtomicU64)>,
  pub(crate) downloaded_files: Arc<(AtomicU64, AtomicU64)>,
  pub(crate) downloaded_bytes: Arc<(AtomicU64, AtomicU64)>,
  pub(crate) patched_files: Arc<(AtomicU64, AtomicU64)>,
  pub(crate) patched_bytes: Arc<(AtomicU64, AtomicU64)>,
}