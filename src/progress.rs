use async_trait::async_trait;
use std::sync::{Arc, Mutex};

use crate::update::Update;

pub struct DownloadProgress {
    pub global_progress: Option<Arc<Mutex<Progress>>>
}

impl DownloadProgress {
    pub fn new(global_progress: Arc<Mutex<Progress>>) -> Self {
        Self {
            global_progress: Some(global_progress)
        }
    }
}

#[async_trait]
impl download_async::Progress for DownloadProgress {
    async fn get_file_size(&self) -> usize {
        64
    }

    async fn get_progess(&self) -> usize {
        64
    }

    async fn set_file_size(&mut self, size: usize) {
        
    }

    async fn add_to_progress(&mut self, amount: usize) {
        if let Some(global_progress) = self.global_progress.as_deref() {
            let mut state = global_progress.lock().unwrap();
            state.download_size.0 += amount as u64;
            drop(state);
        }
    }

    async fn remove_from_progress(&mut self, amount: usize) {
        if let Some(global_progress) = self.global_progress.as_deref() {
            let mut state = global_progress.lock().unwrap();
            state.download_size.0 -= amount as u64;
            drop(state);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Progress {
  pub update: Update,
  pub hashes_checked: (u64, u64),
  pub download_size: (u64,u64), //Downloaded .. out of .. bytes
  pub patch_files: (u64, u64), //Patched .. out of .. files
  pub finished_hash: bool,
  pub finished_patching: bool,
}

impl Progress {
    pub fn new() -> Progress {
      Progress {
        update: Update::Unknown,
        hashes_checked: (0,0),
        download_size: (0,0),
        patch_files: (0,0),
        finished_hash: false,
        finished_patching: false,
      }
    }
  }
