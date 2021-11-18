use crate::{pausable::PausableTrait};
use crate::structures::{Error, Mirrors, Progress};
use crate::functions::{parse_instructions, retrieve_instructions};

pub async fn flow(mut mirrors: Mirrors, game_location: String, instructions_hash: String, progress_callback: Box<dyn Fn(&Progress) + Send>) -> Result<(), Error> {
  let progress = Progress::new();
  progress.set_current_action("Testing mirrors!".to_string())?;
  progress_callback(&progress);
  mirrors.test_mirrors().await?;

  progress.set_current_action("Downloading instructions!".to_string())?;
  progress_callback(&progress);

  // Download Instructions.json
  let instructions = retrieve_instructions(instructions_hash, &mirrors).pausable().await?;
  
  progress.set_current_action("Parsing instructions!".to_string())?;
  progress_callback(&progress);

  // Parse Instructions.json
  let instructions = parse_instructions(instructions)?;

  println!("{:#?}", instructions);

  Ok(())
}