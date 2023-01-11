use std::sync::Arc;

use crate::{Progress, pausable::{FutureContext, PausableTrait}, structures::{Mirrors, Instruction}, Error};

use super::{parse_instructions, retrieve_instructions};

pub(crate) async fn download_instructions(mut mirrors: Mirrors, instructions_hash: &str, progress: Progress, progress_callback: Box<dyn Fn(&Progress) + Send>, context: Arc<FutureContext>) -> Result<(Vec<Instruction>, Box<dyn Fn(&Progress) + Send>), Error> {
    progress.set_current_action("Testing mirrors!".to_string())?;
    progress_callback(&progress);
    mirrors.test_mirrors().await?;
    
    progress.set_current_action("Downloading instructions file!".to_string())?;
    progress_callback(&progress);
    
    // Download Instructions.json
    let instructions = retrieve_instructions(instructions_hash, &mirrors).pausable(context.clone()).await?;
    
    progress.set_current_action("Parsing instructions file!".to_string())?;
    progress_callback(&progress);
    
    // Parse Instructions.json
    Ok((parse_instructions(instructions)?, progress_callback))
}