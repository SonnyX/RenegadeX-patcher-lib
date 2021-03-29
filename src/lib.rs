extern crate rayon;
extern crate json;
extern crate sha2;
extern crate ini;
extern crate hex;
extern crate num_cpus;
extern crate futures;
extern crate tokio;
extern crate url;
extern crate runas;
extern crate log;
extern crate download_async;
extern crate async_trait;

//Modules
mod apply;
mod directory;
mod downloader;
mod download_entry;
mod error;
//mod filesystem;
mod hashes;
mod instruction_group;
mod instructions;
mod mirrors;
pub mod patcher;
pub mod patcher_builder;
mod pausable;
mod patch_entry;
mod progress;
mod tests;
pub mod traits;
mod update;
mod utilities;

pub use crate::patcher::Patcher;
pub use crate::traits::Error;

static global_runtime : Option<tokio::runtime::Runtime> = None;
static patcher : Option<Patcher> = None;

/// patcher::stop()
pub fn stop() -> Result<(), Error> {
  Ok(())
}

/// patcher::start()
pub fn start() -> Result<(), Error> {
  Ok(())
}
/// patcher::resume()
pub fn resume() -> Result<(), Error> {
  Ok(())
}

/// patcher::pause()
pub fn pause() -> Result<(), Error> {
  Ok(())
}



/*
/// public api might want to be looking as follows:
/// 
/// patcher::PatcherBuilder::new()
/// builder.set_mirrors_url();
/// builder.initialize_patcher();
/// 
/// patcher::get_version_information();
/// 
/// patcher::start()
/// patcher::stop()
/// patcher::resume()
/// patcher::pause()
/// 
/// patcher::get_progress();
/// patcher::
/// patcher::remove_unversioned()
/// 
/// 


Copying of files comes first?
Think of renames, we should process these before downloading!
target_hash goes to null

after sorting the files into groups

download_patch_file() -> patch_file_location
let patch_entry = PatchEntry::new();
let patched_file = patch_file(patch_entry)
for remaining files: copy_file(patched_file, target_file);

*/
