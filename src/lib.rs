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

//Modules

/*
public api might want to be looking as follows:

let builder = patcher::PatcherBuilder::new()
builder.set_mirrors_url();
let patcher = builder.build();

patcher.get_version_information();

patcher.start()
patcher.stop()
patcher.resume()
patcher.pause()

patcher.get_progress();
patcher.
patcher.remove_unversioned()



Copying of files comes first?
Think of renames, we should process these before downloading!
target_hash goes to null

after sorting the files into groups

download_patch_file() -> patch_file_location
let patch_entry = PatchEntry::new();
let patched_file = patch_file(patch_entry)
for remaining files: copy_file(patched_file, target_file);

*/
