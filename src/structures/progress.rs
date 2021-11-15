pub struct Progress {
  current_action: String,
  verified_files: (u64, u64),
  downloaded_files: (u64, u64),
  downloaded_bytes: (u64, u64),
  patched_files: (u64,u64),
  patched_bytes: (u64, u64),
}