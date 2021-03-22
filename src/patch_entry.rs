#[derive(Debug,Clone)]
pub struct PatchEntry {
  /// Path to target file
  target_path: String,
  /// path to patch file
  delta_path: String,
  /// If the patch file requires a target file
  has_source: bool,
  /// The expected target hash after patching
  target_hash: String,
}