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
mod directory;
mod downloader;
mod error;
mod instructions;
mod mirrors;
pub mod patcher;
pub mod traits;
mod pausable;
mod instruction_group;
mod hashes;
mod filesystem;
mod progress;
mod tests;