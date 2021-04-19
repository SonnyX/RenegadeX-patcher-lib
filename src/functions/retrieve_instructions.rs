use std::io::Write;
use std::time::Duration;

use crate::structures::{Error, Mirrors, Instruction};
use crate::functions::download_file;
use crate::traits::{ExpectUnwrap,BorrowUnwrap};
use crate::traits::AsString;

use log::warn;
use sha2::{Sha256, Digest};

pub(crate) async fn retrieve_instructions(mirrors: &Mirrors) -> Result<Vec<Instruction>, Error> {
    if mirrors.is_empty() {
      return Err(Error::NoMirrors());
    }
    // todo: rewrite to have a race to fetch instructions the fastest
    let mut instructions : String = "".to_string();
    for retry in 0_usize..3_usize {
      let mirror = mirrors.get_mirror();
      let result : Result<(),Error> = {
        let url = format!("{}/instructions.json", &mirror.address);
  
        let mut text = download_file(url, Duration::from_secs(60)).await?;
        let bytes = text.as_ref();
        // check instructions hash
        let mut sha256 = Sha256::new();
        sha256.write(&bytes)?;
        let hash = hex::encode_upper(sha256.finalize());
        if &hash != mirrors.instructions_hash.borrow() {
          Err(Error::HashMismatch(hash, mirrors.instructions_hash.borrow().clone()))
        } else {
          instructions = text.text()?;
          Ok(())
        }
      };
      if result.is_ok() {
        break;
      } else if result.is_err() && retry == 2 {
        //TODO: This is bound to one day go wrong
        return Err(Error::OutOfRetries("Couldn't fetch instructions.json"));
      } else {
        warn!("Removing mirror: {:#?}", &mirror);
        mirrors.remove(mirror);
      }
    }
    let instructions_data = match json::parse(&instructions) {
      Ok(result) => result,
      Err(e) => return Err(Error::InvalidJson("instructions.json".to_string(), instructions))
    };
    let mut instructions = Vec::with_capacity(instructions_data.len());
    instructions_data.into_inner().iter().for_each(|instruction| {
      instructions.push(Instruction {
        path:                 instruction["Path"].as_string().replace("\\", "/"),
        previous_hash:        instruction["OldHash"].as_string_option(),
        newest_hash:          instruction["NewHash"].as_string_option(),
        full_vcdiff_hash:     instruction["CompressedHash"].as_string_option(),
        delta_vcdiff_hash:    instruction["DeltaHash"].as_string_option(),
        full_vcdiff_size:     instruction["FullReplaceSize"].as_usize().unexpected(""),
        delta_vcdiff_size:    instruction["DeltaSize"].as_usize().unexpected(""),
        has_delta:            instruction["HasDelta"].as_bool().unexpected("")
      });
    });
    Ok(instructions)
  }