use std::time::Duration;

use futures::{StreamExt};
use futures::stream::FuturesUnordered;

use crate::{pausable::PausableTrait};
use crate::structures::{Error, Mirrors, Progress, Instruction};
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

  let mut futures : Box<FuturesUnordered<_>> = Box::new(FuturesUnordered::new());
  progress.set_instructions_amount(instructions.len().try_into().expect("Somehow we have more than 2^64 instructions, colour me impressed"));
  let handle = tokio::runtime::Handle::current();


  for instruction in instructions {
    let mirrors = mirrors.clone();
    let progress = progress.clone();
    futures.push(handle.spawn(process_instruction(instruction, mirrors, progress)));
  }
  progress.set_current_action("Validating, Downloading, Patching!".to_string())?;
  progress_callback(&progress);


  let (future, abort_handle) = futures::future::abortable(async move {
    loop {
      tokio::time::sleep(Duration::from_millis(250)).await;
      progress_callback(&progress);
    }
  });
  handle.spawn(future);
  loop {
    match futures.next().await {
      Some(handle) => {
        match handle {
          Ok(Ok(instruction)) => {
            println!("downloaded {}", instruction.path);
          },
          Ok(Err(e)) => {
            eprintln!("futures.next() returned: {}", e);
          },
          Err(e) => {
            eprintln!("futures.next() returned: {}", e);
          },
        };
      },
      None => {
        println!("Done!");
        break;
      }
    }
  }
  abort_handle.abort();
  //let futures = futures::future::try_join_all(futures).await;
  //let progress_update = futures::future::abortable(progress.call_every(Duration::from_millis(250)));

  //futures::future::select(futures, progress_update.0).await;
  Ok(())
}

async fn process_instruction(instruction: Instruction, mirrors: Mirrors, progress: Progress) -> Result<Instruction, Error> {
  let action = instruction.determine_action().await?;
      progress.increment_processed_instructions();
      
      match action {
        crate::structures::Action::DownloadFull => {
          let file = instruction.newest_hash.clone().expect("Download full, but there's no full vcdiff hash");
          download_file_in_parallel("full", file, instruction.full_vcdiff_size, mirrors, progress).await?;

          //apply_patch(instruction.path, instruction.full_vcdiff_hash, instruction.full_vcdiff_hash, false);
          //progress.increment_patched_done();
          Ok(instruction)
        },
        crate::structures::Action::DownloadDelta => {
          let file = format!("{}_from_{}", &instruction.newest_hash.clone().expect("Download delta, but there's no newest hash"), &instruction.previous_hash.clone().expect("Download delta, but there's no previous hash"));
          download_file_in_parallel("delta", file, instruction.delta_vcdiff_size, mirrors, progress).await?;

          //apply_patch(instruction.path, instruction.full_vcdiff_hash, instruction.full_vcdiff_hash, true);
          Ok(instruction)
        },
        crate::structures::Action::Nothing => {
          Ok(instruction)
        },
      }
}