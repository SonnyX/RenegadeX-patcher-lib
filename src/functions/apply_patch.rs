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
pub(crate) async fn apply_patch(target_path: String, target_hash: String, delta_path: String) -> Result<(), Error> {
  let mut dir_path = target_path.clone();
  dir_path.truncate(target_path.rfind('/').ok_or_else(|| Error::None(format!("{} contains no /", target_path)))?);
  // Create directory incase it does not exist
  DirBuilder::new().recursive(true).create(dir_path)?;

  let target_path_clone = target_path.clone();

  tokio::task::spawn_blocking(move || {
    if std::fs::File::open(&target_path_clone).is_ok() {
      // If the patch_entry is a delta

      let source_path = format!("{}.vcdiff_src", &target_path_clone);
      std::fs::rename(&target_path_clone, &source_path)?;
      xdelta::decode_file(Some(&source_path), &delta_path, &target_path_clone);
      std::fs::remove_file(&source_path)?;
    } else {
      // If the patch_entry is a full
      xdelta::decode_file(None, &delta_path, &target_path_clone);
    }
    Ok::<(), Error>(())
  }).await??;
  let hash = get_hash(&target_path).await?;
  if hash != target_hash {
    return Err(Error::HashMismatch(target_path.clone(), hash, target_hash.clone()));
  }
  Ok(())
}