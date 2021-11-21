use std::time::Duration;

use futures::future::join_all;

use crate::{pausable::PausableTrait};
use crate::structures::{Error, Mirrors, Progress};
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

  let mut futures = vec!();
  for instruction in instructions {
    let mirrors = mirrors.clone();
    let progress = progress.clone();
    futures.push(async move {
      let action = instruction.determine_action().await?;
      match action {
        crate::structures::Action::DownloadFull => {
          download_file_in_parallel("full", instruction.full_vcdiff_hash.expect("Download full, but there's no full vcdiff hash"), mirrors, progress).await?;

          //apply_patch(instruction.path, instruction.full_vcdiff_hash, instruction.full_vcdiff_hash, false);
          //progress.increment_patched_done();
          Ok::<(), Error>(())
        },
        crate::structures::Action::DownloadDelta => {
          //download_file_in_parallel("delta", instruction.full_vcdiff_hash.expect("Download full, but there's no full vcdiff hash"), mirrors, progress).await?;

          //apply_patch(instruction.path, instruction.full_vcdiff_hash, instruction.full_vcdiff_hash, true);
          Ok::<(), Error>(())
        },
        crate::structures::Action::Nothing => {
          Ok::<(), Error>(())
        },
      }
    });
  }
  let futures = futures::future::try_join_all(futures).await;
  //let progress_update = futures::future::abortable(progress.call_every(Duration::from_millis(250)));

  //futures::future::select(futures, progress_update.0).await;
  Ok(())
}