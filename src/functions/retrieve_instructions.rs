use std::io::Write;
use std::time::Duration;

use crate::structures::{Error, Mirrors};

use log::warn;
use sha2::{Sha256, Digest};

pub(crate) async fn retrieve_instructions(instructions_hash: String, mirrors: &Mirrors) -> Result<String, Error> {
  if mirrors.is_empty() {
    return Err(Error::NoMirrors());
  }
  // todo: rewrite to have a race to fetch instructions the fastest
  let mut instructions : String = "".to_string();
  for retry in 0_usize..3_usize {
    let mirror = mirrors.get_mirror()?;
    let result : Result<(),Error> = {
      let mut text = mirror.download_file(format!("instructions.json"), Duration::from_secs(60)).await?;
      let bytes = text.as_ref();
      // check instructions hash
      let mut sha256 = Sha256::new();
      sha256.write(&bytes)?;
      let hash = hex::encode_upper(sha256.finalize());
      if &hash != &instructions_hash {
        Err(Error::HashMismatch(format!("{}/instructions.json", mirror.address), hash, instructions_hash.clone()))
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
  return Ok(instructions);
}