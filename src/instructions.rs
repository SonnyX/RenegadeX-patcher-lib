use crate::traits::Error;
use crate::mirrors::Mirrors;
use std::sync::Mutex;
use log::*;
use std::time::Duration;
use sha2::{Sha256, Digest};
use crate::downloader::download_file;
use crate::traits::*;
use std::io::Write;

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
  /// SHA256 hash of this file during current patch, None if the file is to be deleted
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



pub(crate) async fn retrieve_instructions(mirrors: &Mirrors, instructions: &mut Vec<Instruction>, renegadex_location: &str) -> Result<(), Error> {
  if mirrors.is_empty() {
    return Err("No mirrors found! Did you retrieve mirrors?".to_string().into());
  }
  if !instructions.is_empty() {
    return Ok(());
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
      sha256.write(&bytes);
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
  let instructions_text : String = instructions_mutex.into_inner().unexpected("");
  let instructions_data = match json::parse(&instructions_text) {
    Ok(result) => result,
    Err(e) => return Err(format!("Invalid JSON: {}", e).into())
  };
  instructions_data.into_inner().iter().for_each(|instruction| {
    let file_path = format!("{}{}", renegadex_location, instruction["Path"].as_string().replace("\\", "/"));
    instructions.push(Instruction {
      path:                 file_path,
      previous_hash:        instruction["OldHash"].as_string_option(),
      newest_hash:          instruction["NewHash"].as_string_option(),
      full_vcdiff_hash:     instruction["CompressedHash"].as_string_option(),
      delta_vcdiff_hash:    instruction["DeltaHash"].as_string_option(),
      full_vcdiff_size:     instruction["FullReplaceSize"].as_usize().unexpected(""),
      delta_vcdiff_size:    instruction["DeltaSize"].as_usize().unexpected(""),
      has_delta:            instruction["HasDelta"].as_bool().unexpected("")
    });
  });
  Ok(())
}