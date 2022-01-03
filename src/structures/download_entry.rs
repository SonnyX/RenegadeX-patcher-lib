#[derive(Debug)]
pub struct DownloadEntry {
  /// The path relative to a mirror
  pub mirror_path: String,
  /// The path of the downloaded file
  pub download_path: String,
  /// The expected size of the downloaded file
  pub download_size: u64,
  /// The expected hash of the downloaded file
  pub download_hash: String,
  /// Path to target file
  pub target_path: String,
  /// The expected target hash after patching
  pub target_hash: String
}