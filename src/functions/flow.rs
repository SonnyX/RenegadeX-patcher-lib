use std::io::SeekFrom;
use std::time::Duration;
use futures::{StreamExt};
use futures::stream::FuturesUnordered;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use crate::{pausable::PausableTrait};
use crate::structures::{Error, Mirrors, Progress, Instruction, Action, Mirror, Response};
use crate::functions::{download_file_in_parallel, parse_instructions, retrieve_instructions};

pub async fn flow(mut mirrors: Mirrors, game_location: String, instructions_hash: String, progress_callback: Box<dyn Fn(&Progress) + Send>) -> Result<(), Error> {
  let progress = Progress::new();
  progress.set_current_action("Testing mirrors!".to_string())?;
  progress_callback(&progress);
  mirrors.test_mirrors().await?;
  
  progress.set_current_action("Downloading instructions file!".to_string())?;
  progress_callback(&progress);
  
  // Download Instructions.json
  let instructions = retrieve_instructions(instructions_hash, &mirrors).pausable().await?;
  
  progress.set_current_action("Parsing instructions file!".to_string())?;
  progress_callback(&progress);
  
  // Parse Instructions.json
  let mut instructions = parse_instructions(instructions)?;
  instructions.sort_by(|a, b| a.full_vcdiff_size.cmp(&b.full_vcdiff_size));
  
  progress.set_current_action("Processing instructions!".to_string())?;
  progress_callback(&progress);
  
  progress.set_instructions_amount(instructions.len().try_into().expect("Somehow we have more than 2^64 instructions, colour me impressed"));
  let handle = tokio::runtime::Handle::current();
  tokio::fs::create_dir_all(format!("{}/patcher/", &game_location)).await?;
  let mut actions = FuturesUnordered::new();
  for instruction in &instructions {
    actions.push(async move { (instruction.clone(), instruction.determine_action().await) });
  }
  progress.set_current_action("Validating, Downloading, Patching!".to_string())?;
  progress_callback(&progress);

  let progress_clone = progress.clone();
  let (future, abort_handle) = futures::future::abortable(async move {
    loop {
      tokio::time::sleep(Duration::from_millis(250)).await;
      progress_callback(&progress_clone);
    }
  });
  handle.spawn(future);



  let (sender, mut receiver) = tokio::sync::mpsc::channel::<(Action, Instruction, String, String, Vec<(usize, usize, usize)>)>(instructions.len());
  
  let mirrors_clone = mirrors.clone();
  let progress_clone = progress.clone();

  let downloader = handle.spawn(async move {
    let handle = tokio::runtime::Handle::current();
    while let Some((action, instruction, file_location, url, parts)) = receiver.recv().await {
      let mut handlers = Box::new(FuturesUnordered::new());
      for (part, from, to) in parts {
        let mirror = mirrors_clone.get_mirror()?;
        let url = url.clone();
        let to = to.clone();
        let from = from.clone();
        let future = handle.spawn(async move {
          (part, from, download_part(url, mirror, from, to).await)
        });
        handlers.push(future);
      }
      let mut f = OpenOptions::new().read(true).write(true).create(true).open(&file_location).await?;
      let size = match action {
        Action::DownloadFull => instruction.full_vcdiff_size,
        Action::DownloadDelta => instruction.delta_vcdiff_size,
        Action::Nothing => todo!(),
    };
      loop {
        match handlers.next().await {
          Some(handle) => {
            match handle {
              Ok((part, from, Ok(response))) => {
                f.seek(SeekFrom::Start(from as u64)).await?;
                f.write_all(&mut response.as_ref()).await?;
                f.seek(SeekFrom::Start((size + part) as u64)).await?;
                f.write(&[1_u8]).await?;
                progress_clone.increment_downloaded_bytes(response.as_ref().len() as u64);
                log::info!("downloaded {} for {}", part, &file_location);
              },
              Ok((part, _, Err(e))) => {
                log::error!("download_part {} returned: {}", part, e);
              },
              Err(e) => {
                log::error!("handlers.next() returned: {}", e);
              },
            };
          },
          None => {
            log::info!("Done!");
            break;
          }
        }
      }
    }
    Ok::<(), Error>(())
  });
  loop {
    match actions.next().await {
      Some(handle) => {
        let mirrors = mirrors.clone();
        let progress = progress.clone();
        let game_location = game_location.clone();
        
        match handle {
          (instruction, Ok(action)) => {
            progress.increment_processed_instructions();
            let size = match action {
              crate::structures::Action::DownloadFull => instruction.full_vcdiff_size,
              crate::structures::Action::DownloadDelta => instruction.delta_vcdiff_size,
              crate::structures::Action::Nothing => continue,
            };
            let folder = match action {
              Action::DownloadFull => "full",
              Action::DownloadDelta => "delta",
              Action::Nothing => continue,
          };
            let file = match action {
                Action::DownloadFull => instruction.newest_hash.clone().expect("Download full, but there's no full vcdiff hash"),
                Action::DownloadDelta => format!("{}_from_{}", &instruction.newest_hash.clone().expect("Download delta, but there's no newest hash"), &instruction.previous_hash.clone().expect("Download delta, but there's no previous hash")),
                Action::Nothing => continue,
            };
            //progress.add_download(size as u64);
            //download_file_in_parallel(folder, file, size, game_location, mirrors, progress).await?;
            // start download_file_in_parallel
            const PART_SIZE : usize = 2u64.pow(20) as usize; //1.048.576 == 1 MB aprox
            let file_location = format!("{}/patcher/{}", game_location, &file);
            log::info!("Opening: {}", &file_location);
            let mut f = OpenOptions::new().read(true).write(true).create(true).open(&file_location).await?;
            //set the size of the file, add a byte for each part to the end of the file as a means of tracking progress.
            let parts_amount : usize = size / PART_SIZE + if size % PART_SIZE > 0 {1} else {0};
            let file_size : usize = size + parts_amount;
            f.set_len(file_size as u64).await?;

            //We have set up the file
            log::info!("Seeking to location of {}", &file_location);
            f.seek(SeekFrom::Start(size as u64)).await?;
            let mut completed_parts = vec![0; parts_amount];
            f.read_exact(&mut completed_parts).await?;
            
            let download_parts : Vec<usize> = completed_parts.iter().enumerate().filter(|(i, part)| part == &&0_u8).map(|(i,_)| i).collect();
            let mut parts  = vec![];
            let url = format!("{}/{}", &folder, &file);
          
            let mut download_amount = 0;
            for part in download_parts {
                let from = part*PART_SIZE;
                let mut to = part*PART_SIZE+PART_SIZE;
                if to > size {
                  to = size;
                }
                download_amount += to - from;
                parts.push((part, from, to));
            }
            sender.send((action, instruction, file_location, url, parts)).await.unwrap();
            progress.add_download(download_amount as u64);
            //progress_callback(&progress);

            // end of that
          },
          (instruction, Err(e)) => {
            log::error!("futures.next() returned: {}", e);
          },
        };
        
      },
      None => {
        log::info!("Done!");
        break;
      }
    }
  }
  downloader.await;
  abort_handle.abort();
  Ok(())
}

async fn download_part(url: String, mirror: Mirror, from: usize, to: usize) -> Result<Response, Error> {
  let mut downloader = download_async::Downloader::new();
  let uri = format!("{}/{}", mirror.address, url).parse::<download_async::http::Uri>()?;
  downloader.use_uri(uri);
  downloader.use_sockets(mirror.ip);
  let headers = downloader.headers().expect("Couldn't unwrap download_async headers option");
  headers.append("User-Agent", format!("RenX-Patcher ({})", env!("CARGO_PKG_VERSION")).parse().unwrap());
  headers.append("Range", format!("bytes={}-{}", from, to).parse().unwrap());

  let mut buffer = vec![];
  downloader.allow_http();
  let response = downloader.download(download_async::Body::empty(), &mut buffer);

  let result = tokio::time::timeout(Duration::from_secs(60), response).await??;
  Ok(Response::new(result, buffer))
}