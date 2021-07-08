use std::path::Path;

use crate::functions::get_hash;
use crate::structures::{Error, InstructionGroup};

pub(crate) async fn apply_instructions(group: InstructionGroup) -> Result<(), Error> {
  let hash = group.hash.ok_or_else(|| Error::None(format!("")))?;
    // First hash the most important candidates in group
    let mut file_path_1 = None;
    for file_path in group.previous_hash_matches {
        if Path::new(&file_path).exists() {
            let hash = get_hash(&file_path)?;
            if hash.eq(&hash) {
                file_path_1 = Some(file_path.clone());
                break;
            }
        }
    }

    if let Some(file_path) = file_path_1 {
        // Copy file over to new locations
        for new_file_path in group.current_hash_matches {
            tokio::fs::copy(&file_path, new_file_path).await?;
        }
        return Ok(());
    }

    // - e.g. File gets changed

    // Rename files

    // Copy over files

    /*
       Same old hash as old hash
     - Doesn't really matter unless one of them is missing!!!
    Same old hash as new hash
     - Copy files over if it exists duh :)
     - Update the old hash to a newer hash afterwards if necessary...
    Same new hash as new hash
     - Copy files over after patch

       */

    Ok(())
}
