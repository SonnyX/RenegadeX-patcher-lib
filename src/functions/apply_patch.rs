use crate::structures::PatchEntry;
use crate::structures::Error;
use std::fs::DirBuilder;
use crate::functions::get_hash;

///
/// Applies the vcdiff patch file to the target file.
/// ```
/// -------------- par --------------------------------------------------
/// | DeltaQueue | --> | apply patch to all files that match this Delta |
/// --------------     --------------------------------------------------
///```
pub(crate) async fn apply_patch(target_path: String, target_hash: String, delta_path: String, has_source: bool) -> Result<(), Error> {
  let mut dir_path = target_path.clone();
  dir_path.truncate(target_path.rfind('/').ok_or_else(|| Error::None(format!("{} contains no /", target_path)))?);
  // Create directory incase it does not exist
  DirBuilder::new().recursive(true).create(dir_path)?;

  
  if has_source {
    // If the patch_entry is a delta

    let source_path = format!("{}.vcdiff_src", &target_path);
    std::fs::rename(&target_path, &source_path)?;
    xdelta::decode_file(Some(&source_path), &delta_path, &target_path);
    std::fs::remove_file(&source_path)?;
  } else {
    // If the patch_entry is a full

    // There is supposed to be no source file, so make sure it doesn't exist either!
    match std::fs::remove_file(&target_path) {
      Ok(()) => (),
      Err(_e) => ()
    };
    xdelta::decode_file(None, &delta_path, &target_path);
  }
  let hash = get_hash(&target_path).await?;
  if hash != target_hash {
    return Err(Error::HashMismatch(target_path.clone(), hash, target_hash.clone()));
  }
  Ok(())
}