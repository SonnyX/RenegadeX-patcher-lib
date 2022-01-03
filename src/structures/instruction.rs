/// An instruction
#[derive(Debug, Clone)]
pub(crate) struct Instruction {
  /// Path to which the instruction applies
  pub path: String,
  /// SHA256 hash of this file during the previous patch, None if this is a new file
  pub previous_hash: Option<String>,
  /// SHA256 hash of this file during current patch, None if the file is to be deleted/moved
  pub newest_hash: Option<String>,
  /// SHA256 hash of Full vcdiff patch file
  pub full_vcdiff_hash: Option<String>,
  /// SHA256 hash of Delta vcdiff patch file
  pub delta_vcdiff_hash: Option<String>,
  /// Size of `Full` vcdiff patch file
  pub full_vcdiff_size: u64,
  /// Size of `Delta` vcdiff patch file
  pub delta_vcdiff_size: u64,
  /// Does file have a Delta vcdiff patch file
  pub has_delta: bool
}