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
pub(crate) fn apply_patch(patch_entry: &PatchEntry) -> Result<(), Error> {
    let mut dir_path = patch_entry.target_path.clone();
    dir_path.truncate(patch_entry.target_path.rfind('/').ok_or_else(|| Error::None(format!("{} contains no /", patch_entry.target_path)))?);
    // Create directory incase it does not exist
    DirBuilder::new().recursive(true).create(dir_path)?;

    
    if patch_entry.has_source {
      // If the patch_entry is a delta

      let source_path = format!("{}.vcdiff_src", &patch_entry.target_path);
      std::fs::rename(&patch_entry.target_path, &source_path)?;
      xdelta::decode_file(Some(&source_path), &patch_entry.delta_path, &patch_entry.target_path);
      std::fs::remove_file(&source_path)?;
    } else {
      // If the patch_entry is a full

      // There is supposed to be no source file, so make sure it doesn't exist either!
      match std::fs::remove_file(&patch_entry.target_path) {
        Ok(()) => (),
        Err(_e) => ()
      };
      xdelta::decode_file(None, &patch_entry.delta_path, &patch_entry.target_path);
    }
    let hash = get_hash(&patch_entry.target_path)?;
    if hash != patch_entry.target_hash {
      return Err(Error::HashMismatch(patch_entry.target_path.clone(), hash, patch_entry.target_hash.clone()));
    }
    Ok(())
  }