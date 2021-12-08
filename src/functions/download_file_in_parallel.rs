use std::{io::SeekFrom, time::Duration};

use crate::structures::Response;
use log::info;
use tokio::{fs::OpenOptions, io::{AsyncSeekExt, AsyncReadExt}, task::JoinHandle};

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
  let mut handlers : Vec<(usize, JoinHandle<Result<Response, Error>>)> = vec![];
  let url = format!("{}/{}", &folder, &url);

  for part in download_parts {
      let handle = tokio::runtime::Handle::current();
      let from = part*PART_SIZE;
      let mut to = part*PART_SIZE+PART_SIZE;
      if to > size {
        to = size;
      }
      let future = handle.spawn(download_part(url.clone(), mirrors.clone(), from, to));
      handlers.push((part, future));
  }

  for (part, handle) in handlers {
    match handle.await {
      Ok(_) => {},
      Err(e) => todo!(),
    };
  }
  Ok(())
}

async fn download_part(url: String, mirrors: Mirrors, from: usize, to: usize) -> Result<Response, Error> {
  let mut downloader = download_async::Downloader::new();
  downloader.use_uri(url.parse::<download_async::http::Uri>()?);
  let headers = downloader.headers().expect("Couldn't unwrap download_async headers option");
  headers.append("User-Agent", format!("RenX-Patcher ({})", env!("CARGO_PKG_VERSION")).parse().unwrap());
  headers.append("Range", format!("bytes={}-{}", from, to).parse().unwrap());

  let mut buffer = vec![];
  downloader.allow_http();
  let response = downloader.download(download_async::Body::empty(), &mut buffer);

  let result = tokio::time::timeout(Duration::from_secs(60), response).await??;
  Ok(Response::new(result, buffer))
}

/*
  ///
  /// Downloads the file in parts
  ///
  ///
  async fn download_file(&self, mirror: &Mirror, download_url: &str, download_entry: &DownloadEntry, first_attempt: bool) -> Result<(), Error> {
    //set the size of the file, add a 32bit integer to the end of the file as a means of tracking progress. We won't download parts async.
    

    self.get_file(&mirror, f, &download_url, resume_part, part_size, &download_entry).await?;
    //Let's make sure the downloaded file matches the Hash found in Instructions.json
    let hash = get_hash(&download_entry.file_path)?;
    if hash != download_entry.file_hash {
      let mut state = self.state.lock().unexpected("");
      state.download_size.0 -= download_entry.file_size as u64;
      drop(state);
      return Err(format!("File \"{}\"'s hash ({}) did not match with the one provided in Instructions.json ({})", &download_entry.file_path, &hash, &download_entry.file_hash).into());
    }
    Ok(())
  }


  pub async fn get_download_file(unlocked_state: Arc<Mutex<Progress>>, mirror: &Mirror, f: std::fs::File, download_url: &str, resume_part: usize, part_size: usize, download_entry: &DownloadEntry ) -> Result<(), Error>  {
  let mut writer = BufWriter::new(f, move | file, total_written | {
    //When the buffer is being written to file, this closure gets executed
    let parts = *total_written / part_size as u64;
    file.seek(SeekFrom::End(-4)).unexpected("");
    file.write_all(&(parts as u32).to_be_bytes()).unexpected("");
    file.seek(SeekFrom::Start(*total_written)).unexpected("");
  });
  writer.seek(SeekFrom::Start((part_size * resume_part) as u64)).unexpected("");

  let url = download_url.parse::<download_async::http::Uri>().unexpected("");
  let trunc_size = download_entry.file_size as u64;

  let mut req = download_async::http::Request::builder();
  req = req.uri(url).header("User-Agent", "sonny-launcher/1.0");
  if resume_part != 0 {
    req = req.header("Range", format!("bytes={}-{}", (part_size * resume_part), download_entry.file_size));
  };
  let req = req.body(download_async::Body::empty()).unexpected("");
  let ip = mirror.ip.clone();

  let mut progress = crate::progress::DownloadProgress::new(unlocked_state);

  let result = download_async::download(req, &mut writer, false, &mut Some(&mut progress), Some(ip)).await;

  if result.is_ok() {
    let f = writer.into_inner()?;
    f.sync_all()?;
    f.set_len(trunc_size)?;
    Ok(())
  } else {
    Err(format!("Unexpected response: found status code!").into())
  }
}
*/