use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::structures::mirrors::Mirrors;
use crate::downloader::download_file;
use crate::traits::{ExpectUnwrap,Error,BorrowUnwrap};
use crate::traits::AsString;

use log::*;
use sha2::{Sha256, Digest};

pub(crate) async fn retrieve_instructions(mirrors: &Mirrors) -> Result<Vec<Instruction>, Error> {
    if mirrors.is_empty() {
      return Err("No mirrors found! Did you retrieve mirrors?".to_string().into());
    }
  
    let instructions_mutex : Mutex<String> = Mutex::new("".to_string());
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
          Err(format!("Hash of instructions.json ({}) did not match the one specified in release.json ({})!", &hash, mirrors.instructions_hash.borrow()).into())
        } else {
          *instructions_mutex.lock().unexpected("") = text.text()?;
          Ok(())
        }
      };
      if result.is_ok() {
        break;
      } else if result.is_err() && retry == 2 {
        //TODO: This is bound to one day go wrong
        return Err("Couldn't fetch instructions.json".to_string().into());
      } else {
        warn!("Removing mirror: {:#?}", &mirror);
        mirrors.remove(mirror);
      }
    }
    let instructions_text : String = instructions_mutex.into_inner().map_err( | error_message | { Error::from(error_message) })?;
    let instructions_data = match json::parse(&instructions_text) {
      Ok(result) => result,
      Err(e) => return Err(format!("Invalid JSON: {}", e).into())
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