use std::ffi::OsString;
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Sender, Receiver};

struct Chunk {
  /// The file the chunk belongs to
  file: OsString,
  /// The part containing the data and the starting position of this data.
  part: Part,
  /// Is this chunk the last chunk of this file?
  is_last: bool
}
struct Part {
  start_location: usize,
  data: Vec<u8>,
}

// Set up a singular task which takes care of writing data to disk
struct FileSystem {
  parts: BTreeMap<OsString, VecDeque<Part>>,
  use_memory_only: bool,
}


// what I want to happen is:
// Have a loop somewhere that is responsible for receiving parts from elsewhere, add them to a hashmap.
// Have a loop somewhere that is responsible for writing parts of which the most is available, however, we don't want to run all the time.
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