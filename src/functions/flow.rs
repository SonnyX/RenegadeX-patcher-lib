use futures::channel::mpsc::{UnboundedSender, UnboundedReceiver};
use log::{info, error};
use tokio::sync::Mutex;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use crate::Error;

use futures::StreamExt;
use futures::TryStreamExt;
use futures::FutureExt;

use crate::functions::delete_file;
use crate::functions::determine_parts_to_download;
use crate::pausable::PausableTrait;
use crate::structures::DownloadEntry;
use crate::structures::FilePart;
use crate::structures::{Mirrors, Progress, Action};
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
  let (tell_to_complete, mut should_complete_receiver) = futures::channel::oneshot::channel::<()>();

  let future = async move {
    loop {
      let should_complete = should_complete_receiver.try_recv();
      if should_complete.is_err() || should_complete.ok().is_some() {
        break;
      }
      tokio::time::sleep(Duration::from_millis(250)).await;
      progress_callback(&repeated_progress);
    }
    info!("Done reporting download/patching progress!");
    progress_callback
  };
  let handle = tokio::runtime::Handle::current();
  let join_handle = handle.spawn(future);
  
  let actions = futures::stream::iter(instructions).map(|instruction| instruction.determine_action(game_location.clone())).buffer_unordered(10);

  // Increment the progress and filter out Action::Nothing
  
  let actions = actions
  .inspect_ok(|action| {
    progress.increment_processed_instructions();
    if let Action::Download(_) = action {
      progress.add_to_be_patched();
    } 
  })
  .filter(|action_result| futures::future::ready(match action_result { Ok(Action::Nothing)  => false, _ => true }));

  let delete_file_tasks : Vec<Pin<Box<dyn futures::Future<Output = Result<(), Error>> + Send + Sync>>> = vec![];
  let (sender, receiver) = futures::channel::mpsc::unbounded();
  let tracker_lock : Arc<Mutex<HashMap<String, (Vec<crate::structures::DownloadEntry>, Vec<u64>)>>> = Arc::new(Mutex::new(HashMap::new()));
  
  let (patching_sender, mut patching_receiver) = futures::channel::mpsc::unbounded();

  let actions_fut = verify_files(sender, game_location.clone(), actions, progress.clone(), patching_sender.clone(), tracker_lock.clone(), delete_file_tasks, mirrors);

  let downloads_fut = download_files(receiver, progress.clone(), tracker_lock.clone(), patching_sender);

  let progress_clone = progress.clone();
  let patching_fut = actions_fut.then(|validation_result| async move {
    loop {
      if let Some(patching_entry) = patching_receiver.next().await {
        info!("Patching target file: {}, using the file {}", &patching_entry.target_path, &patching_entry.download_path);
        apply_patch(patching_entry.target_path, patching_entry.target_hash, patching_entry.download_path).await?;
        progress_clone.increment_completed_patches();
      } else {
        info!("Done patching files!");
        break;
      }
    }
    validation_result
  });
  let (patching_result, downloads_result) = futures::join!(patching_fut, downloads_fut);
  
  tell_to_complete.send(()).expect("Couldn't tell the progress future to complete");

  downloads_result?;
  patching_result?;

  info!("No errors for downloading or patching");
  
  let progress_callback = join_handle.await?;

  info!("Progress join handle was awaited");

  // process_instruction: 1 at a time?
  // download_parts: num of mirrors * 2?
  // Write part to file: 1 after process_instruction is done
  // patch_file: 1 after process_instruction is done, same queue as write part to file

  progress.set_current_action("Cleaning up files".to_string())?;
  progress_callback(&progress);

  info!("Set progress (Cleaning up files)");

  std::fs::remove_dir_all(format!("{}patcher", &game_location))?;

  Ok(())
}

async fn verify_files(
  sender: UnboundedSender<Pin<Box<dyn futures::Future<Output = Result<(FilePart, Vec<u8>), Error>> + Send>>>,
  game_location: String,
  mut actions: impl StreamExt<Item = Result<Action, Error>> + Unpin,
  progress: Progress,
  patching_sender: UnboundedSender<DownloadEntry>,
  tracker_lock: Arc<Mutex<HashMap<String, (Vec<crate::structures::DownloadEntry>, Vec<u64>)>>>,
  mut delete_file_tasks: Vec<Pin<Box<dyn futures::Future<Output = Result<(), Error>> + Send + Sync>>>,
  mirrors: Mirrors
) -> Result<(), Error> {
  let patcher_folder = format!("{}patcher", &game_location);
  std::fs::DirBuilder::new().recursive(true).create(patcher_folder)?;

  loop {
    if let Some(action) = actions.next().await {
      if let Ok(action) = action {
        info!("action: {:#?}", action);
        match action {
            Action::Download(download_entry) => {
              let mut exists = false;
              let mut tracker = tracker_lock.lock().await;
              if let Some((download_entries, parts)) = tracker.get_mut(&download_entry.download_path) {
                if parts.len() == 0 {
                  info!("Ey, can start patchin this file: {:#?}", &download_entry);
                  progress.add_ready_to_patch();
                  patching_sender.unbounded_send(download_entry.clone()).expect("Closed or sum shit");
                } else {
                  download_entries.push(download_entry.clone());
                }
                exists = true;
              }
              drop(tracker);

              if exists {
                continue;
              }

              let (download_location, parts) = determine_parts_to_download(&download_entry.download_path, &download_entry.download_hash, download_entry.download_size).await?;
              if parts.len() == 0 {
                let f = std::fs::OpenOptions::new().read(true).write(true).open(&download_entry.download_path)?;
                f.set_len(download_entry.download_size)?;
                drop(f);
                info!("Ey, can start patchin this file: {:#?}", &download_entry);
                progress.add_ready_to_patch();
                patching_sender.unbounded_send(download_entry.clone()).expect("Closed or sum shit");
              } else {
                progress.add_download(parts.iter().map(|part| part.to - part.from).sum());
                // add parts to be downloaded
                parts.iter().for_each(|part| sender.unbounded_send(Box::pin(part.clone().download(mirrors.clone(), download_entry.mirror_path.clone()))).expect("Channel closed or something"));
                let mut tracker = tracker_lock.lock().await;
                let mut vec = Vec::new();
                vec.push(download_entry);
                tracker.insert(download_location, (vec, parts.iter().map(|part| part.part_byte).collect::<Vec<u64>>()));
                drop(tracker);
                // when parts are downloaded, patch file
              }
            },
            Action::Delete(file) => delete_file_tasks.push(Box::pin(delete_file(file))),
            Action::Nothing => {},
        };
      } else if let Err(e) = action {
        error!("Processing file into action failed: {:#?}", e);
      }
    } else {
      info!("Done verifying files!");
      break;
    }
  }
  drop(sender);
  drop(patching_sender);
  Ok::<(), Error>(())
}

async fn download_files(
  receiver: UnboundedReceiver<Pin<Box<dyn futures::Future<Output = Result<(FilePart, Vec<u8>), Error>> + Send>>>,
  progress: Progress,
  tracker_lock: Arc<Mutex<HashMap<String, (Vec<crate::structures::DownloadEntry>, Vec<u64>)>>>,
  patching_sender: UnboundedSender<DownloadEntry>,
) -> Result<(), Error> {
  let mut buffered_receiver = receiver.buffer_unordered(10);
  loop {
    if let Some(action) = buffered_receiver.next().await {
      if let Ok((part, buffer)) = action {
        info!("Part downloaded: {:#?}", part);
        part.write_to_file(buffer).await?;
        progress.increment_downloaded_bytes(part.to - part.from);

        let mut tracker = tracker_lock.lock().await;
        let (download_entries, parts) = tracker.get_mut(&part.file).ok_or_else(|| Error::None(format!("No tracker entry found for: {}", &part.file)))?;
        parts.remove(parts.binary_search(&part.part_byte).expect("Could not find the part_byte"));
        if parts.len() == 0 {
          let f = std::fs::OpenOptions::new().read(true).write(true).open(&download_entries[0].download_path)?;
          f.set_len(download_entries[0].download_size)?;
          drop(f);
          progress.increment_completed_downloads();

          download_entries.iter().for_each(|download_entry| {
            info!("Ey, can start patchin this file: {:#?}", &download_entry);
            progress.add_ready_to_patch();
            patching_sender.unbounded_send(download_entry.clone()).expect("Closed or sum shit");
          });
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
  drop(patching_sender);
  Ok::<(), Error>(())
}