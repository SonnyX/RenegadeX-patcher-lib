use std::ffi::OsString;
use std::collections::BTreeMap;
use tokio::sync::mpsc::Receiver;
use std::collections::btree_map::Entry;
use crossbeam_queue::SegQueue;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::futures::Stream;

struct ChunkHeader {
  file: OsString,
  size: usize,
}

struct Chunk {
  /// The file the chunk belongs to
  file: OsString,
  /// The part containing the data and the starting position of this data.
  part: Part,
}

struct Part {
  start_location: usize,
  data: Vec<u8>,
}

struct File {
  size: usize,
  done: bool,
  queue: SegQueue<Part>,
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
  parts: BTreeMap<OsString, File>,
  use_memory_only: bool,
  receiver: Receiver<Chunk>,
  file_initializer: Receiver<ChunkHeader>,
  keep_going: AtomicBool,
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
        let receiver = self.receivers.get(most_backed_up);

      }
    }
  }
}