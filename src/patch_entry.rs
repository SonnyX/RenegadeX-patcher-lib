#[derive(Debug,Clone)]
pub struct PatchEntry {
  /// Path to target file
  pub target_path: String,
  /// path to patch file
  pub delta_path: String,
  /// If the patch file requires a target file
  pub has_source: bool,
  /// The expected target hash after patching
  pub target_hash: String,
}