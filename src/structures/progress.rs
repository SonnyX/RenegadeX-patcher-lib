use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Progress {
  pub(crate) current_action: Arc<Mutex<String>>,
  pub processed_instructions: Arc<(AtomicU64, AtomicU64)>,
  pub downloaded_files: Arc<(AtomicU64, AtomicU64)>,
  pub downloaded_bytes: Arc<(AtomicU64, AtomicU64)>,
  pub patched_files: Arc<(AtomicU64, AtomicU64)>,
  pub patched_bytes: Arc<(AtomicU64, AtomicU64)>,
}