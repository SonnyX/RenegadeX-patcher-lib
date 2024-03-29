use futures::channel::mpsc::{UnboundedSender, UnboundedReceiver};
use tracing::{Instrument, instrument};
use tracing::{info, error};
use tokio::sync::Mutex;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::Error;

use futures::StreamExt;
use futures::TryStreamExt;
use futures::FutureExt;

use crate::functions::delete_file;
use crate::functions::determine_parts_to_download;
use crate::pausable::{PausableTrait, FutureContext};
use crate::structures::{DownloadEntry, Instruction};
use crate::structures::FilePart;
use crate::structures::{Mirrors, Progress, Action};
use crate::functions::apply_patch;


pub(crate) async fn flow(mirrors: Mirrors, game_location: &String, instructions: Vec<Instruction>, progress: Progress, progress_callback: Box<dyn Fn(&Progress) + Send>, context: Arc<FutureContext>) -> Result<Box<dyn Fn(&Progress) + Send>, Error> {
  progress.set_instructions_amount(instructions.len().try_into().expect("Somehow we have more than 2^64 instructions, colour me impressed"));
  progress.set_current_action("Validating, Downloading, Patching!".to_string())?;
  progress_callback(&progress);

  let repeated_progress = progress.clone();
  let report_progress = Arc::new(std::sync::atomic::AtomicBool::new(true));
  let report_progress_clone = report_progress.clone();

  let future = async move {
    loop {
      if report_progress_clone.load(Ordering::Relaxed) == false {
        break;
      }
      tokio::time::sleep(Duration::from_millis(250)).instrument(tracing::info_span!("Progress callback sleep")).await;
      progress_callback(&repeated_progress);
    }
    info!("Done reporting download/patching progress!");
    progress_callback
  }.instrument(tracing::info_span!("Progress callback loop"));
  let handle = tokio::runtime::Handle::current();
  let progress_handle = tokio::task::Builder::new().name("Progress loop").spawn_on(future, &handle)?.instrument(tracing::info_span!("Progress callback loop"));
  let game_location_clone = game_location.clone();
  let actions = futures::stream::iter(instructions).map(move |instruction| instruction.determine_action(game_location_clone.clone())).buffer_unordered(1);

  // Increment the progress and filter out Action::Nothing
  let progress_clone = progress.clone();

  let actions = actions
  .inspect_ok(move |action| {
    progress_clone.increment_processed_instructions();
    if let Action::Download(_) = action {
      progress_clone.add_to_be_patched();
    } 
  })
  .filter(|action_result| futures::future::ready(match action_result { Ok(Action::Nothing)  => false, _ => true }));

  let delete_file_tasks : Vec<Pin<Box<dyn futures::Future<Output = Result<(), Error>> + Send + Sync>>> = vec![];
  let (sender, receiver) = futures::channel::mpsc::unbounded();
  let tracker_lock : Arc<Mutex<HashMap<String, (Vec<crate::structures::DownloadEntry>, Vec<u64>)>>> = Arc::new(Mutex::new(HashMap::new()));
  
  let (patching_sender, mut patching_receiver) = futures::channel::mpsc::unbounded();

  let actions_fut = verify_files(sender, game_location.clone(), actions, progress.clone(), patching_sender.clone(), tracker_lock.clone(), delete_file_tasks, mirrors.clone());
  let actions_handle = tokio::task::Builder::new().name("Verification loop").spawn_on(actions_fut.pausable(context.clone()), &handle)?;

  let downloads_fut = download_files(receiver, progress.clone(), tracker_lock.clone(), patching_sender).instrument(tracing::info_span!("Download loop"));

  let progress_clone = progress.clone();
  let patching_fut = actions_handle.then(|validation_result| async move {
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
    validation_result?
  }.instrument(tracing::info_span!("Patching loop")));

  info!("Gonna wait for patching and downloading to be done");

  let (patching_result, downloads_result) = futures::join!(tokio::task::Builder::new().name("Actions/Patching loop").spawn_on(patching_fut.pausable(context.clone()), &handle)?.instrument(tracing::info_span!("Patching loop")), tokio::task::Builder::new().name("Download loop").spawn_on(downloads_fut.pausable(context.clone()), &handle)?);
  
  info!("Patching and downloading done, telling progress to quit");

  let _ = report_progress.store(false, Ordering::Relaxed);

  info!("Told progress to quit");

  downloads_result??;

  info!("No download errors");

  patching_result??;

  info!("No patching errors");
  
  let progress_callback = progress_handle.await?;

  info!("Progress join handle was awaited");

  // process_instruction: 1 at a time?
  // download_parts: num of mirrors * 2?
  // Write part to file: 1 after process_instruction is done
  // patch_file: 1 after process_instruction is done, same queue as write part to file

  progress.set_current_action("Cleaning up files".to_string())?;
  progress_callback(&progress);

  info!("Set progress (Cleaning up files)");

  std::fs::remove_dir_all(format!("{}patcher", &game_location))?;

  Ok(progress_callback)
}

#[instrument(skip(sender, actions, progress, delete_file_tasks))]
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

              let (download_location, parts) = determine_parts_to_download(&download_entry.download_path, &download_entry.download_hash, download_entry.download_size)?;
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
                parts.iter().for_each(|part| sender.unbounded_send(Box::pin(part.clone().download(mirrors.clone(), download_entry.mirror_path.clone(), progress.clone()))).expect("Channel closed or something"));
                let mut tracker = tracker_lock.lock().await;
                let mut vec = Vec::new();
                vec.push(download_entry);
                tracker.insert(download_location, (vec, parts.iter().map(|part| part.part_byte).collect::<Vec<u64>>()));
                drop(tracker);
                // when parts are downloaded, patch file
              }
            },
            Action::Delete(file) => delete_file_tasks.push(Box::pin(async move { delete_file(file) })),
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

#[instrument(skip(receiver, progress_original))]
async fn download_files(
  receiver: UnboundedReceiver<Pin<Box<dyn futures::Future<Output = Result<(FilePart, Vec<u8>), Error>> + Send>>>,
  progress_original: Progress,
  tracker_lock: Arc<Mutex<HashMap<String, (Vec<crate::structures::DownloadEntry>, Vec<u64>)>>>,
  patching_sender_original: UnboundedSender<DownloadEntry>,
) -> Result<(), Error> {
  let mut buffered_receiver = receiver.buffer_unordered(10);
  loop {
    if let Some(action) = buffered_receiver.next().await {
      let tracker_lock_clone = tracker_lock.clone();
      let progress = progress_original.clone();
      let patching_sender = patching_sender_original.clone();

      if let Ok((part, buffer)) = action {
        tokio::task::Builder::new().name(&format!("Handling part {} of {}", part.part_byte, part.file)).spawn(async move {
            info!("Part downloaded: {:#?}", part);
            part.write_to_file(buffer).await?;
            //progress.increment_downloaded_bytes(part.to - part.from);
    
            let mut tracker = tracker_lock_clone.lock().await;
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
          Ok::<(), Error>(())
        })?.await??;
      } else if let Err(e) = action {
        error!("Downloading FilePart failed: {:#?}", e);
      }
    } else {
      info!("Done downloading files!");
      break;
    }
  }
  drop(patching_sender_original);
  Ok::<(), Error>(())
}