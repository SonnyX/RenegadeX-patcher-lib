use std::{io::SeekFrom};

use crate::structures::{FilePart};
use tokio::{fs::OpenOptions, io::{AsyncSeekExt, AsyncReadExt, AsyncWriteExt}};

use crate::{structures::{Error}, functions::get_hash};

pub async fn determine_parts_to_download(file_name: &str, file_hash: &str, size: u64, game_location: &str) -> Result<(String, Vec<FilePart>), Error> {
  const PART_SIZE : u64 = 2u64.pow(20); //1.048.576 == 1 MB aprox
  let file_location = format!("{}/patcher/{}", game_location, &file_name);
  log::info!("Opening: {}", &file_location);
  let mut f = OpenOptions::new().read(true).write(true).create(true).open(&file_location).await?;
  //set the size of the file, add a byte for each part to the end of the file as a means of tracking progress.
  let parts_amount : u64 = size / PART_SIZE + if size % PART_SIZE > 0 {1} else {0};
  let file_size : u64 = size + parts_amount;
  log::info!("Getting metadata of {}", &file_location);
  let file_metadata = f.metadata().await?;
  if (file_metadata.len()) != file_size {
    if file_metadata.len() == size {
      //If hash is correct, return.
      //Otherwise download again.
      log::info!("Getting hash of {}", &file_location);
      let hash = get_hash(&file_location).await?;
      if hash == file_hash {
        return Ok((file_location, vec!()));
      }
    }
    log::info!("Setting size of {}", &file_location);
    f.set_len(file_size as u64).await?;
    f.flush().await?;
  }
  //We have set up the file
  log::info!("Seeking to location of {}", &file_location);
  f.seek(SeekFrom::Start(size as u64)).await?;
  let mut completed_parts = vec![0; parts_amount as usize];
  f.read_exact(&mut completed_parts).await?;
  f.flush().await?;
  
  let download_parts : Vec<FilePart> = completed_parts.iter().enumerate().filter(|(i, part)| part == &&0_u8).map(|(i,_)| FilePart::new(file_location.clone(), size + (i as u64), ( i as u64 ) * PART_SIZE, ( ( (i + 1) as u64) * PART_SIZE).min(size))).collect();
  return Ok((file_location, download_parts));
}


/*
#[cfg(test)]
mod my_tests {
  use crate::structures::*;
  use crate::functions::*;
  use tokio::fs::create_dir;

  pub trait AsString {
    fn as_string(&self) -> String;
  }
  impl AsString for json::JsonValue {
    fn as_string(&self) -> String {
      match *self {
        json::JsonValue::Short(ref value)  => value.to_string(),
        json::JsonValue::String(ref value) => value.to_string(),
        _ => {
          panic!("Expected a JSON String, however got: {}", self.dump())
        }
      }
    }
  }


  #[tokio::test(flavor = "multi_thread", worker_threads = 6)]
  pub async fn myTest() -> Result<(),Error> {

      let mut downloader = download_async::Downloader::new();
      downloader.use_uri("https://static.ren-x.com/launcher_data/version/release.json".parse::<download_async::http::Uri>()?);
      let headers = downloader.headers().expect("Couldn't unwrap download_async headers option");
      headers.append("User-Agent", format!("RenX-Patcher ({})", env!("CARGO_PKG_VERSION")).parse().unwrap());
      let mut buffer = vec![];
      downloader.allow_http();
      let response = downloader.download(download_async::Body::empty(), &mut buffer);

      let _ = tokio::time::timeout(std::time::Duration::from_secs(10), response).await??;

      let file = String::from_utf8(buffer)?;
      let parsed_json = json::parse(&file)?;
      let named_urls : Vec<crate::NamedUrl> = parsed_json["game"]["mirrors"].members().map(|json| crate::NamedUrl {
          name: json["name"].as_string(),
          url: json["url"].as_string(),
      }).collect();
      println!("{:#?}", &named_urls);

      let mut mirrors = Mirrors::new(named_urls, parsed_json["game"]["patch_path"].as_string());

      let progress = crate::Progress::new();
      mirrors.test_mirrors().await?;

      let instructions = retrieve_instructions(&parsed_json["game"]["instructions_hash"].as_string(), &mirrors).await?;
      let mut instructions = parse_instructions(instructions)?;
      instructions.sort_by(|a,b| a.full_vcdiff_size.cmp(&b.full_vcdiff_size));
      let instruction = instructions.pop().ok_or(Error::None(format!("No instructions")))?;
      println!("{:#?}", &instruction);
      let _ = create_dir(format!("patcher")).await;
      let file = determine_parts_to_download("full", &instruction.newest_hash.ok_or(Error::None(format!("No newest_hash")))?, instruction.full_vcdiff_size, format!("../"), mirrors, progress).await?;
      Ok(())
  }
}
*/