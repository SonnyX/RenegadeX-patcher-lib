use crate::error::Error;
use crate::mirrors::Mirrors;
use std::sync::Mutex;
use log::*;
use std::time::Duration;
use sha2::{Sha256, Digest};
use crate::downloader::download_file;
use crate::traits::*;

pub struct HashMember {
  pub path: String,
  pub previous_hash: Option<String>,

}



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
  pub full_vcdiff_size: usize,
  /// Size of `Delta` vcdiff patch file
  pub delta_vcdiff_size: usize,
  /// Does file have a Delta vcdiff patch file
  pub has_delta: bool
}


#[derive(Debug,Clone)]
pub struct PatchEntry {
  /// Path to the target
  target_path: String,
  /// Path to the vcdiff file
  delta_path: String,
  /// If the target_path needs to apply on something? idk
  has_source: bool,
  target_hash: String,
}



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
      let text = text.text()?;
      // check instructions hash
      let mut sha256 = Sha256::new();
      sha256.input(&text);
      let hash = hex::encode_upper(sha256.result());
      if &hash != mirrors.instructions_hash.borrow() {
        Err(format!("Hash of instructions.json ({}) did not match the one specified in release.json ({})!", &hash, mirrors.instructions_hash.borrow()).into())
      } else {
        *instructions_mutex.lock().unexpected(concat!(module_path!(),":",file!(),":",line!())) = text;
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
  let instructions_text : String = instructions_mutex.into_inner().map_err(error_message || { error_message.message = format!("", error_message) })?;
  let instructions_data = match json::parse(&instructions_text) {
    Ok(result) => result,
    Err(e) => return Err(format!("Invalid JSON: {}", e).into())
  };
  let instructions = Vec::with_capacity(instructions_data.len());
  instructions_data.into_inner().iter().for_each(|instruction| {
    instructions.push(Instruction {
      path:                 instruction["Path"].as_string().replace("\\", "/"),
      previous_hash:        instruction["OldHash"].as_string_option(),
      newest_hash:          instruction["NewHash"].as_string_option(),
      full_vcdiff_hash:     instruction["CompressedHash"].as_string_option(),
      delta_vcdiff_hash:    instruction["DeltaHash"].as_string_option(),
      full_vcdiff_size:     instruction["FullReplaceSize"].as_usize().unexpected(concat!(module_path!(),":",file!(),":",line!())),
      delta_vcdiff_size:    instruction["DeltaSize"].as_usize().unexpected(concat!(module_path!(),":",file!(),":",line!())),
      has_delta:            instruction["HasDelta"].as_bool().unexpected(concat!(module_path!(),":",file!(),":",line!()))
    });
  });
  Ok(instructions)
}