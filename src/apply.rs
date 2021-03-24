use crate::patch_entry::PatchEntry;
use crate::progress::Progress;
use std::sync::{Arc, Mutex};
use crate::traits::ExpectUnwrap;
use crate::traits::Error;
use std::fs::DirBuilder;
use crate::hashes::get_hash;

///
/// Applies the vcdiff patch file to the target file.
/// ```
/// -------------- par --------------------------------------------------
/// | DeltaQueue | --> | apply patch to all files that match this Delta |
/// --------------     --------------------------------------------------
///```
pub(crate) fn apply_patch(patch_entry: &PatchEntry, state: Arc<Mutex<Progress>>) -> Result<(), Error> {
    let mut dir_path = patch_entry.target_path.clone();
    dir_path.truncate(patch_entry.target_path.rfind('/').unexpected(""));
    // Create directory incase it does not exist
    DirBuilder::new().recursive(true).create(dir_path).unexpected("");

    
    if patch_entry.has_source {
      // If the patch_entry is a delta

      let source_path = format!("{}.vcdiff_src", &patch_entry.target_path);
      std::fs::rename(&patch_entry.target_path, &source_path).unexpected("");
      xdelta::decode_file(Some(&source_path), &patch_entry.delta_path, &patch_entry.target_path);
      std::fs::remove_file(&source_path).unexpected("");
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
      return Err(format!("Hash for file {} is incorrect!\nGot hash: {}\nExpected hash: {}", &patch_entry.target_path, &hash, &patch_entry.target_hash).into());
    }
    let mut state = state.lock().unexpected(concat!(module_path!(),":",file!(),":",line!()));
    state.patch_files.0 += 1;
    drop(state);
    Ok(())
  }