use futures::FutureExt;
use log::{info, error};
use tokio::sync::Mutex;
use std::collections::HashMap;
use std::time::Duration;

use futures::StreamExt;
use futures::TryStreamExt;

use crate::functions::delete_file;
use crate::functions::determine_parts_to_download;
use crate::pausable::PausableTrait;
use crate::structures::{Error, Mirrors, Progress, Action};
use crate::functions::{apply_patch, parse_instructions, retrieve_instructions};


pub async fn flow(mut mirrors: Mirrors, game_location: String, instructions_hash: &str, progress_callback: Box<dyn Fn(&Progress) + Send>) -> Result<(), Error> {
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
  let instructions = parse_instructions(instructions)?;
  
  progress.set_current_action("Processing instructions!".to_string())?;
  progress_callback(&progress);
  
  //let mut futures : Box<FuturesUnordered<_>> = Box::new(FuturesUnordered::new());
  progress.set_instructions_amount(instructions.len().try_into().expect("Somehow we have more than 2^64 instructions, colour me impressed"));
  progress.set_current_action("Validating, Downloading, Patching!".to_string())?;
  progress_callback(&progress);

  let repeated_progress = progress.clone();
  let (future, abort_handle) = futures::future::abortable(async move {
    loop {
      tokio::time::sleep(Duration::from_millis(250)).await;
      progress_callback(&repeated_progress);
    }
  });
  let handle = tokio::runtime::Handle::current();
  handle.spawn(future);
  
  let actions = futures::stream::iter(instructions).map(|instruction| instruction.determine_action(game_location.clone())).buffer_unordered(10);

  // Increment the progress and filter out Action::Nothing
  
  let mut actions = actions
  .inspect_ok(|_| progress.increment_processed_instructions())
  .filter(|action_result| futures::future::ready(match action_result { Ok(Action::Nothing)  => false, _ => true }));

  let mut delete_file_tasks = vec![];
  let (sender, receiver) = futures::channel::mpsc::unbounded();
  let tracker_lock = Mutex::new(HashMap::new());
  
  let (patching_sender, mut patching_receiver) = futures::channel::mpsc::unbounded();

  //let downloads = downloads.buffered(10);
  let actions_fut = async {
    let patcher_folder = format!("{}patcher", &game_location);
    std::fs::DirBuilder::new().recursive(true).create(patcher_folder)?;

    loop {
      if let Some(action) = actions.next().await {
        if let Ok(action) = action {
          info!("action: {:#?}", action);
          match action {
              Action::Download(download_entry) => {
                let (download_location, parts) = determine_parts_to_download(&download_entry.download_path, &download_entry.download_hash, download_entry.download_size).await?;
                if parts.len() == 0 {
                  info!("Ey, can start patchin this file: {:#?}", &download_entry);
                  patching_sender.unbounded_send(download_entry.clone()).expect("Closed or sum shit");
                } else {
                  progress.add_download(parts.iter().map(|part| part.to - part.from).sum());
                  // add parts to be downloaded
                  parts.iter().for_each(|part| sender.unbounded_send(part.clone().download(mirrors.clone(), download_entry.mirror_path.clone())).expect("Channel closed or something"));
                  let mut tracker = tracker_lock.lock().await;
                  tracker.insert(download_location, (download_entry, parts.iter().map(|part| part.part_byte).collect::<Vec<u64>>()));
                  drop(tracker);
                  // when parts are downloaded, patch file
                }
              },
              Action::Delete(file) => delete_file_tasks.push(delete_file(file)),
              Action::Nothing => {},
          };
        } else if let Err(e) = action {
          error!("Processing file into action failed: {:#?}", e);
        }
      } else {
        break;
      }
    }
    Ok::<(), Error>(())
  };

  let downloads_fut = async {
    let mut buffered_receiver = receiver.buffer_unordered(10);
    loop {
      if let Some(action) = buffered_receiver.next().await {
        if let Ok((part, buffer)) = action {
          info!("Part downloaded: {:#?}", part);
          part.write_to_file(buffer).await?;
          progress.increment_downloaded_bytes(part.to - part.from);

          let mut tracker = tracker_lock.lock().await;
          let (download_entry, parts) = tracker.get_mut(&part.file).ok_or_else(|| Error::None(format!("No tracker entry found for: {}", &part.file)))?;
          parts.remove(parts.binary_search(&part.part_byte).expect(""));
          if parts.len() == 0 {
            info!("Ey, can start patchin this file: {:#?}", &download_entry);
            patching_sender.unbounded_send(download_entry.clone()).expect("Closed or sum shit");
          }
          drop(tracker);
        } else if let Err(e) = action {
          error!("Downloading FilePart failed: {:#?}", e);
        }
      } else {
        info!("Done downloading files!");
        break;
      }
    }
    Ok::<(), Error>(())
  };

  let patching_fut = actions_fut.then(|validation_result| async move { 
    loop {
      if let Some(patching_entry) = patching_receiver.next().await {
        info!("Patching target file: {}, using the file {}", &patching_entry.target_path, &patching_entry.download_path);
        apply_patch(patching_entry.target_path, patching_entry.target_hash, patching_entry.download_path).await;
      } else {
        break;
      }
    }
    validation_result
  });
  let (actions_result, downloads_result) = futures::join!(patching_fut, downloads_fut);
  actions_result?;
  downloads_result?;


  // process_instruction: 1 at a time?
  // download_parts: num of mirrors * 2?
  // Write part to file: 1 after process_instruction is done
  // patch_file: 1 after process_instruction is done, same queue as write part to file

  abort_handle.abort();
  Ok(())
}