use std::ffi::OsString;
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Sender, Receiver};

struct Chunk {
  file: OsString,
  part: Part,
}
struct Part {
  start_location: usize,
  data: Vec<u8>,
}

// Set up a singular task which takes care of writing data to disk
struct FileSystem {
  parts: BTreeMap<OsString, VecDeque<Part>>
}

impl FileSystem {
  pub async fn receive_parts(&mut self, mut receiver: Receiver<Chunk>) {
    while let Some(chunk) = receiver.recv().await {
      self.parts.entry(chunk.file).or_insert(VecDeque::new()).push_back(chunk.part);
    }
  }

  async fn get_most_backed_up(&self) -> Option<OsString> {
    self.parts.iter()
    .max_by(|a, b| a.1.len().cmp(&b.1.len()))
    .map(|(k, _v)| k.clone())
  }

  pub async fn write_parts_to_disk(&self) {
    
  }
}