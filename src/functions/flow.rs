use crate::{pausable::PausableTrait, structures::{Error, Mirrors}};
use crate::functions::{parse_instructions, retrieve_instructions};

pub async fn flow(mut mirrors: Mirrors, game_location: String, instructions_hash: String) -> Result<(), Error> {
  mirrors.test_mirrors().await?;

  // Download Instructions.json
  let instructions = retrieve_instructions(instructions_hash, &mirrors).pausable().await?;
  
  // Parse Instructions.json
  let instructions = parse_instructions(instructions)?;

  println!("{:#?}", instructions);

  Ok(())
}