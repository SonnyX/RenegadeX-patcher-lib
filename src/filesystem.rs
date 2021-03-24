use std::ffi::OsString;
use std::collections::BTreeMap;
use tokio::sync::mpsc::Receiver;
use std::collections::btree_map::Entry;
use crossbeam_queue::SegQueue;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::futures::Stream;

struct ChunkHeader {
  pub file: OsString,
  pub size: usize,
}

struct Chunk {
  /// The file the chunk belongs to
  pub file: OsString,
  /// The part containing the data and the starting position of this data.
  pub part: Part,
}

struct Part {
  pub start_location: usize,
  pub data: Vec<u8>,
}

struct File {
  pub size: usize,
  pub done: bool,
  pub queue: SegQueue<Part>,
}

impl File {
  fn new(size: usize) -> Self {
    File {
      size,
      done: false,
      queue: SegQueue::new()
    }
  }
}

// Set up a singular task which takes care of writing data to disk
#[derive(Clone)]
struct FileSystem {
  pub parts: BTreeMap<OsString, File>,
  pub use_memory_only: bool,
  pub receiver: Receiver<Chunk>,
  pub file_initializer: Receiver<ChunkHeader>,
  pub keep_going: AtomicBool,
}


// what I want to happen is:
// Have a loop somewhere that is responsible for receiving parts from elsewhere, add them to a hashmap.
// Have a loop somewhere that is responsible for writing parts of which the most is available, however, we don't want to run all the time.
impl FileSystem {
  pub async fn receive_parts(&mut self) {
    while let Some(chunk) = self.receiver.recv().await {
      match self.parts.get(&chunk.file) {
        Some(file) => {
          file.queue.push(chunk.part);
          
        },
        None => {
          panic!("This shouldn't happen");
        }
      }
    }
  }

  async fn get_most_backed_up(&self) -> Option<OsString> {
    self.parts.iter()
    .max_by(|a, b| a.1.queue.len().cmp(&b.1.queue.len()))
    .map(|(k, _v)| k.clone())
  }

  pub async fn write_parts_to_disk(&self) {
    while self.keep_going.load(Ordering::Relaxed) {
      while let Some(most_backed_up) = self.get_most_backed_up().await {
        let receiver = self.receiver.get(most_backed_up);

      }
    }
  }
}