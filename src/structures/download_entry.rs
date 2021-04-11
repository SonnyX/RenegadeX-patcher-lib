use crate::structures::patch_entry::PatchEntry;

#[derive(Debug)]
pub struct DownloadEntry {
  pub file_path: String,
  pub file_size: usize,
  pub file_hash: String,
  pub patch_entries: Vec<PatchEntry>,
}