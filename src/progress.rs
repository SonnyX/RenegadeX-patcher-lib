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
    fn new() -> Progress {
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