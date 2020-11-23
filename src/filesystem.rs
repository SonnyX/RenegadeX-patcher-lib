use std::ffi::OsString;
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Sender, Receiver};

struct Part {
  start_location: usize,
  data: Vec<u8>,
}

// Set up a singular task which takes care of writing data to disk
struct FileSystem {
  parts: BTreeMap<OsString, VecDeque<Part>>
}

impl FileSystem {
  pub fn do_work(&self, receiver: Receiver<Part>) {
    
  }
}