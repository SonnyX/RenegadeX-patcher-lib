use std::time::Duration;

use futures::StreamExt;
use futures::TryStreamExt;

use crate::pausable::PausableTrait;
use crate::structures::{Error, Mirrors, Progress, Action};
use crate::functions::{parse_instructions, retrieve_instructions};

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
  let mut instructions = parse_instructions(instructions)?;
  instructions.sort_by(|a, b| a.full_vcdiff_size.cmp(&b.full_vcdiff_size));
  
  progress.set_current_action("Processing instructions!".to_string())?;
  progress_callback(&progress);
  
  //let mut futures : Box<FuturesUnordered<_>> = Box::new(FuturesUnordered::new());
  progress.set_instructions_amount(instructions.len().try_into().expect("Somehow we have more than 2^64 instructions, colour me impressed"));
  progress.set_current_action("Validating, Downloading, Patching!".to_string())?;
  progress_callback(&progress);
  
  let actions = futures::stream::iter(instructions).map(|instruction| instruction.determine_action(game_location.clone())).buffer_unordered(10);

  // Increment the progress and filter out Action::Nothing
  
  let mut actions = actions
  .inspect_ok(|_| progress.increment_processed_instructions())
  .filter(|action_result| futures::future::ready(match action_result { Ok(Action::Nothing)  => false, _ => true }));

  // 
  //let parts = actions.
  loop {
    if let Some(action) = actions.next().await {
      log::info!("action: {:#?}", action)
    } else {
      break;
    }
  }

  // process_instruction: 1 at a time?
  // download_parts: num of mirrors * 2?
  // Write part to file: 1 after process_instruction is done
  // patch_file: 1 after process_instruction is done, same queue as write part to file
  
  
  let (future, abort_handle) = futures::future::abortable(async move {
    loop {
      tokio::time::sleep(Duration::from_millis(250)).await;
      progress_callback(&progress);
    }
  });
  let handle = tokio::runtime::Handle::current();
  handle.spawn(future);
  
  abort_handle.abort();

  Ok(())
}