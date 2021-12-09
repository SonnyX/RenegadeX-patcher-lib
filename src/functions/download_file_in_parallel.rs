use std::{io::SeekFrom, time::Duration};

use crate::structures::{Response, Mirror};
use futures::{stream::FuturesUnordered, StreamExt};
use tokio::{fs::OpenOptions, io::{AsyncSeekExt, AsyncReadExt, AsyncWriteExt}, task::JoinHandle};

use crate::{structures::{Error, Mirrors, Progress}, functions::get_hash};

pub async fn download_file_in_parallel(folder: &str, url: String, size: usize, mirrors: Mirrors, progress: Progress) -> Result<(), Error> {
  const PART_SIZE : usize = 2u64.pow(20) as usize; //1.048.576 == 1 MB aprox
  let file_location = format!("patcher/{}", &url);
  let mut f = OpenOptions::new().read(true).write(true).create(true).open(&file_location).await?;
  //set the size of the file, add a byte for each part to the end of the file as a means of tracking progress.
  let parts_amount : usize = size / PART_SIZE + if size % PART_SIZE > 0 {1} else {0};
  let file_size : usize = size + parts_amount;
  let file_metadata = f.metadata().await?;
  if (file_metadata.len() as usize) < file_size {
    if file_metadata.len() == (size as u64) {
      //If hash is correct, return.
      //Otherwise download again.
      let hash = get_hash(&file_location).await?;
      if hash == url {
        return Ok(());
      }
    }
    f.set_len(file_size as u64).await?;
  }
  //We have set up the file
  f.seek(SeekFrom::Start(size as u64)).await?;
  let mut completed_parts = vec![0; parts_amount];
  f.read_exact(&mut completed_parts).await?;
  
  let download_parts : Vec<usize> = completed_parts.iter().enumerate().filter(|(i, part)| part == &&0_u8).map(|(i,_)| i).collect();
  let mut handlers : Box<FuturesUnordered<JoinHandle<(usize, Result<Response, Error>)>>> = Box::new(FuturesUnordered::new());
  let url = format!("{}/{}", &folder, &url);

  let handle = tokio::runtime::Handle::current();
  for part in download_parts {
      let from = part*PART_SIZE;
      let mut to = part*PART_SIZE+PART_SIZE;
      if to > size {
        to = size;
      }
      let url = url.clone();
      let mirror = mirrors.get_mirror()?;
      let future = handle.spawn(async move {
        (part, download_part(url, mirror, from, to).await)
      });
      handlers.push(future);
  }
  loop {
    match handlers.next().await {
      Some(handle) => {
        match handle {
          Ok((part, Ok(response))) => {
            f.seek(SeekFrom::Start((part*PART_SIZE) as u64)).await?;
            f.write_all(&mut response.as_ref()).await?;
            f.seek(SeekFrom::Start((size + part) as u64)).await?;
            f.write(&[1_u8]).await?;
            println!("downloaded {}", part);
          },
          Ok((part, Err(e))) => {
            eprintln!("download_part {} returned: {}", part, e);
          },
          Err(e) => {
            eprintln!("join_error returned: {}", e);
          },
        };
      },
      None => {
        println!("Done!");
        break;
      }
    }
  }
  Ok(())
}

async fn download_part(url: String, mirror: Mirror, from: usize, to: usize) -> Result<Response, Error> {
  let mut downloader = download_async::Downloader::new();
  let uri = format!("{}/{}", mirror.address, url).parse::<download_async::http::Uri>()?;
  downloader.use_uri(uri);
  let headers = downloader.headers().expect("Couldn't unwrap download_async headers option");
  headers.append("User-Agent", format!("RenX-Patcher ({})", env!("CARGO_PKG_VERSION")).parse().unwrap());
  headers.append("Range", format!("bytes={}-{}", from, to).parse().unwrap());

  let mut buffer = vec![];
  downloader.allow_http();
  let response = downloader.download(download_async::Body::empty(), &mut buffer);

  let result = tokio::time::timeout(Duration::from_secs(60), response).await??;
  Ok(Response::new(result, buffer))
}

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

  #[tokio::test]
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

      let instructions = retrieve_instructions(parsed_json["game"]["instructions_hash"].as_string(), &mirrors).await?;
      let mut instructions = parse_instructions(instructions)?;
      instructions.sort_by(|a,b| a.full_vcdiff_size.cmp(&b.full_vcdiff_size));
      let instruction = instructions.pop().ok_or(Error::None(format!("No instructions")))?;
      println!("{:#?}", &instruction);
      let _ = create_dir(format!("patcher")).await;
      let file = download_file_in_parallel("full", instruction.newest_hash.ok_or(Error::None(format!("No newest_hash")))?, instruction.full_vcdiff_size, mirrors, progress).await?;
      Ok(())
  }
}