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
/*
#[derive(Debug,Clone)]
pub struct Instruction {
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
  pub full_vcdiff_size: usize,
  /// Size of `Delta` vcdiff patch file
  pub delta_vcdiff_size: usize,
  /// Does file have a Delta vcdiff patch file
  pub has_delta: bool
}
*/
// JSON example
/*
Same old hash as old hash
 - Doesn't really matter unless one of them is missing!!!
Same old hash as new hash
 - Copy files over if it exists duh :)
 - Update the old hash to a newer hash afterwards if necessary...
Same new hash as new hash
 - Copy files over after patch


{


}






#[derive(Debug)]
pub struct DownloadEntry {
  pub file_path: String,
  pub file_size: usize,
  pub file_hash: String,
  pub patch_entries: Vec<PatchEntry>,
}

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
*/