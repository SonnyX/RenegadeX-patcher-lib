use async_trait::async_trait;
use std::sync::{Arc, Mutex};


pub struct Progress {
    global_progress: Option<Arc<Mutex<crate::patcher::Progress>>>
}

impl Progress {
    pub fn new(global_progress: Arc<Mutex<crate::patcher::Progress>>) -> Self {
        Self {
            global_progress: Some(global_progress)
        }
    }
}

#[async_trait]
impl download_async::Progress for Progress {
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

    async fn remove_from_progress(&mut self, bytes: usize) {
        
    }
}