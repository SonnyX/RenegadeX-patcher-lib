use std::io::Write;
use std::time::Duration;

use crate::structures::{Error, Mirrors, Instruction};
use crate::functions::download_file;
use crate::traits::AsString;

use log::{warn, error};
use sha2::{Sha256, Digest};

pub(crate) async fn retrieve_instructions(mirrors: &Mirrors) -> Result<Vec<Instruction>, Error> {
    if mirrors.is_empty() {
      return Err(Error::NoMirrors());
    }
    // todo: rewrite to have a race to fetch instructions the fastest
    let mut instructions : String = "".to_string();
    for retry in 0_usize..3_usize {
      let mirror = mirrors.get_mirror()?;
      let result : Result<(),Error> = {
        let url = format!("{}/instructions.json", &mirror.address);
  
        let mut text = download_file(url.clone(), Duration::from_secs(60)).await?;
        let bytes = text.as_ref();
        // check instructions hash
        let mut sha256 = Sha256::new();
        sha256.write(&bytes)?;
        let hash = hex::encode_upper(sha256.finalize());
        if &hash != mirrors.instructions_hash.as_ref().ok_or_else(|| Error::None(format!("Couldn't unwrap instructions_hash of the mirrors object")))? {
          Err(Error::HashMismatch(url, hash, mirrors.instructions_hash.as_ref().ok_or_else(|| Error::None(format!("Couldn't unwrap instructions_hash of the mirrors object")))?.clone()))
        } else {
          instructions = text.text()?;
          Ok(())
        }
      };
      if result.is_ok() {
        break;
      } else if retry == 2 {
        //TODO: This is bound to one day go wrong
        return Err(Error::OutOfRetries("Couldn't fetch instructions.json"));
      } else {
        match result.unwrap_err() { // todo: Decide when to remove mirror, and when not to remove it
          Error::DownloadTimeout(e) => {},
          Error::DownloadError(e) => {},
          Error::HttpError(e) => {},
          _ => {}
        };
        warn!("Removing mirror: {:#?}", &mirror);
        mirrors.remove(mirror)?;
      }
    }
    let instructions_data = match json::parse(&instructions) {
      Ok(result) => result,
      Err(e) => return Err(Error::InvalidJson("instructions.json".to_string(), instructions))
    };
    let mut instructions = Vec::with_capacity(instructions_data.len());
    instructions_data.into_inner().iter().for_each(|instruction| {
      let mut closure = || -> Result<(), Error> {
        instructions.push(Instruction {
          path:                 instruction["Path"].as_string().replace("\\", "/"),
          previous_hash:        instruction["OldHash"].as_string_option(),
          newest_hash:          instruction["NewHash"].as_string_option(),
          full_vcdiff_hash:     instruction["CompressedHash"].as_string_option(),
          delta_vcdiff_hash:    instruction["DeltaHash"].as_string_option(),
          full_vcdiff_size:     instruction["FullReplaceSize"].as_usize().ok_or_else(|| Error::None(format!("retrieve_instructions.rs: Could not cast JSON version_number as a usize, input was {}", instruction["FullReplaceSize"])))?,
          delta_vcdiff_size:    instruction["DeltaSize"].as_usize().ok_or_else(|| Error::None(format!("retrieve_instructions.rs: Could not cast JSON version_number as a usize, input was {}", instruction["DeltaSize"])))?,
          has_delta:            instruction["HasDelta"].as_bool().ok_or_else(|| Error::None(format!("retrieve_instructions.rs: Could not cast JSON version_number as a usize, input was {}", instruction["HasDelta"])))?
        });
        Ok(())
      };
      match closure() {
        Ok(()) => {},
        Err(e) => error!("Transforming instructions failed for instruction {}, with error: {}", instruction, e)
      };
    });
    Ok(instructions)
  }