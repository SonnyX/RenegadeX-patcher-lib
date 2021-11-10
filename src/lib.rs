#![feature(slice_group_by)]

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

mod structures;
mod functions;
mod implementations;
mod traits;
mod patcher;
mod patcher_builder;
mod pausable;

pub use patcher::Patcher as Patcher;
pub use patcher_builder::PatcherBuilder as PatcherBuilder;
pub use structures::Error as Error;