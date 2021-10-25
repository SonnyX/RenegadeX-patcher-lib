use crate::structures::Error;
use super::retrieve_instructions;

pub async fn flow() -> Result<(), Error> {

  // Get Instructions from mirror
  let instructions = retrieve_instructions(mirrors).await?;
  // Group instructions into instruction_groups
  instructions.sort_by_cached_key(f);
  // iterate through instruction_groups

  
  
  Ok(())
}