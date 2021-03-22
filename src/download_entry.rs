
#[derive(Debug)]
pub struct DownloadEntry {
  file_path: String,
  file_size: usize,
  file_hash: String,
  patch_entries: Vec<PatchEntry>,
}