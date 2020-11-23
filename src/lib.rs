extern crate rayon;
extern crate json;
extern crate sha2;
extern crate ini;
extern crate hex;
extern crate num_cpus;
extern crate hyper;
extern crate futures;
extern crate tokio;
extern crate url;
extern crate http;
extern crate tower;
extern crate runas;
extern crate log;

//Modules
mod apply;
mod directory;
mod download;
mod downloader;
mod error;
mod instructions;
mod mirrors;
pub mod patcher;
pub mod traits;
mod verify;
mod pausable;
mod instruction_group;
mod hashes;
mod filesystem;

pub use crate::patcher::Patcher;