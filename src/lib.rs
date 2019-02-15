extern crate reqwest;
extern crate rayon;
extern crate json;
extern crate sha2;
extern crate ini;
extern crate hex;

//Standard library
use std::collections::HashMap;
use std::io;
use std::io::{Read, Write, Seek, SeekFrom};
use std::fs::{File,OpenOptions,DirBuilder};
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::panic;

//Modules
mod mirrors;
mod traits;
use mirrors::Mirrors;
use traits::{AsString, BorrowUnwrap};

//External crates
use rayon::prelude::*;
use ini::Ini;
use sha2::{Sha256, Digest};

/*
---------      ------------  par   --------------------
| Entry |  --> | Get Json | ---->  | Try to Open File | 
---------      ------------        --------------------
                                    |                |
                                    |                |
                          ------------------    ----------
                          | Err(Not Found) |    | Ok(()) |
                          ------------------    ----------
                                |                   |
                                |                   |
                   ------------------------   --------------------
                   | Add to DownloadQueue |   | Add to HashQueue |
                   | Add size to size sum |   --------------------
                   | Add to Patch HashMap |
                   ------------------------

DownloadQueue consists of: HashMap of "DownloadFileName":boolean (being downloaded?)
and consists of a FIFO buffer which can be used to 


-------------  par ----------------------     -----------------------
| HashQueue |  --> | Check Hash of File | --> | Compare to OldDelta | 
-------------      ----------------------     -----------------------
                                                |                |
                                                |                |
                                        -------------       ----------
                                        | Different |       |  Same  |
                                        -------------       ----------
                                             |                   |
                                             |                   |
                        ----------------------------------   ------------------------------
                        | Add Full File to DownloadQueue |   | Add Delta to DownloadQueue |
                        |      Add size to size sum      |   |    Add size to size sum    |
                        |      Add to Patch HashMap      |   |    Add to Patch Hashmap    |
                        ----------------------------------   ------------------------------


//add a DownloadQueue bockchain
DeltaQueue needs to be a key-value map, where key is the patch-name, the value would be a Vec<Object>

-------------- par --------------------------------------------------
| DeltaQueue | --> | apply patch to all files that match this Delta |
--------------     --------------------------------------------------
*/

pub struct Progress {
  patch_hashmap: HashMap<String, Vec<PatchEntry>>, //Delta-file-name, instructions that match this
  hash_queue: Vec<Instruction>,
  pub download_size: (u64,u64), //Downloaded .. out of .. bytes
  patch_files: (u64, u64), //Patches .. out of .. files
  pub finished_hash: bool,
}

impl Progress {
  fn new() -> Progress {
    Progress {
      patch_hashmap: HashMap::new(),
      hash_queue: Vec::new(),
      download_size: (0,0),
      patch_files: (0,0),
      finished_hash: false,
    }
  }
}

#[derive(Debug,Clone)]
struct Instruction {
  path: String,
  old_hash: Option<String>,
  new_hash: Option<String>,
  compressed_hash: Option<String>,
  delta_hash: Option<String>,
  full_replace_size: usize,
  delta_size: usize,
  has_delta: bool
}

#[derive(Debug)]
pub struct PatchEntry {
  target_path: String,
  delta_path: String,
  has_source: bool,
  target_hash: String,
}

pub struct Downloader {
  renegadex_location: Option<String>, //Os dependant
  mirrors: Mirrors,
  instructions: Vec<Instruction>, //instructions.json
  pub state: Arc<Mutex<Progress>>,
}


impl Downloader {
  pub fn new() -> Downloader {
    Downloader {
      renegadex_location: None,
      mirrors: Mirrors::new(),
      instructions: Vec::new(),
      state: Arc::new(Mutex::new(Progress::new())),
    }
  }
  pub fn set_location(&mut self, loc: String) {
    self.renegadex_location = Some(loc);
  }
  
  pub fn retrieve_mirrors(&mut self, location: &String) {
    self.mirrors.get_mirrors(location);
  }

  pub fn download(&mut self) {
    if self.mirrors.is_empty() {
      panic!("No mirrors found! Did you retrieve mirrors?");
    }
    if self.instructions.len() == 0 {
      self.retrieve_instructions();
    }
    println!("Retrieved instructions, checking hashes.");
    self.check_hashes();
    //self.download_files();
    {
      let state = self.state.lock().unwrap();
      println!("{:#?}", &state.download_size);
    }
  }
  
  /*
   * Downloads instructions.json from a mirror, checks its validity and passes it on to process_instructions()
   * -------------------------      ------------  par   ------------------------
   * | retrieve_instructions |  --> | Get Json | ---->  | process_instructions | 
   * -------------------------      ------------        ------------------------
  */
  fn retrieve_instructions(&mut self) {
    if self.mirrors.is_empty() {
      panic!("No mirrors found! Did you retrieve mirrors?");
    }
    let instructions_mutex : Mutex<String> = Mutex::new("".to_string());
    for retry in 0..3 {
      let result = std::panic::catch_unwind(|| {
        let instructions_url = format!("{}/instructions.json", &self.mirrors.mirrors[retry].address);
        println!("{}", &instructions_url);
        let mut instructions_response = match reqwest::get(&instructions_url) {
          Ok(result) => result,
          Err(e) => panic!("Is your internet down? {}", e)
        };
        let text = instructions_response.text().unwrap();
        // check instructions hash
        let mut sha256 = Sha256::new();
        sha256.input(&text);
        let hash = hex::encode_upper(sha256.result());
        if &hash != self.mirrors.instructions_hash.borrow() {
          panic!("Hashes did not match!");
        }
        *instructions_mutex.lock().unwrap() = text;
      });
      if result.is_ok() {
        for _i in 0..retry {
          println!("Removing mirror: {:#?}", &self.mirrors.mirrors[0]);
          self.mirrors.remove(0);
        }
        break;
      } else if result.is_err() && retry == 2 {
        panic!("Couldn't fetch instructions.json");
      }
    }
    let instructions_text : String = instructions_mutex.into_inner().unwrap();
    let instructions_data = match json::parse(&instructions_text) {
      Ok(result) => result,
      Err(e) => panic!("Invalid JSON: {}", e)
    };
    self.process_instructions(instructions_data);
  }

  /*
   * ------------------------   par   --------------------
   * | process_instructions |  ---->  | Try to Open File | 
   * ------------------------         --------------------
   *                                   |                |
   *                                   |                |
   *                           ------------------    ----------
   *                           | Err(Not Found) |    | Ok(()) |
   *                           ------------------    ----------
   *                                 |                   |
   *                                 |                   |
   *                    ------------------------   --------------------
   *                    | Add to DownloadQueue |   | Add to HashQueue |
   *                    | Add size to size sum |   --------------------
   *                    | Add to Patch HashMap |
   *                    ------------------------
   * 
   */
  fn process_instructions(&self, instructions_data: json::JsonValue) {
    let mut instruction_array : Vec<Instruction> = Vec::with_capacity(instructions_data.len());
    instructions_data.into_inner().par_iter().for_each(|instruction| {
      //lets start off by trying to open the file.
      let file_path = format!("{}/{}", self.renegadex_location.borrow(), instruction["Path"].as_string().replace("\\", "/")).replace("//","/");
      match OpenOptions::new().read(true).open(&file_path) {
        Ok(file) => {
          if !instruction["NewHash"].is_null() {
            let mut state = self.state.lock().unwrap();
            let hash_entry = Instruction {
              path:                file_path,
              old_hash:            instruction["OldHash"].as_string_option(),
              new_hash:            instruction["NewHash"].as_string_option(),
              compressed_hash:     instruction["CompressedHash"].as_string_option(),
              delta_hash:          instruction["DeltaHash"].as_string_option(),
              full_replace_size:   instruction["FullReplaceSize"].as_usize().unwrap(),
              delta_size:          instruction["DeltaSize"].as_usize().unwrap(),
              has_delta:           instruction["HasDelta"].as_bool().unwrap()
            };
            state.hash_queue.push(hash_entry);
          } else {
            //TODO: DeletionQueue, delete it straight away?
          }
        },
        Err(_e) => {
          if !instruction["NewHash"].is_null() {
            let key = instruction["NewHash"].as_string();
            let delta_path = format!("{}/{}", self.renegadex_location.borrow(), &key).replace("//","/");
            let mut state = self.state.lock().unwrap();
            if !state.patch_hashmap.contains_key(&key) {
              state.patch_hashmap.insert(key.clone(), Vec::new() as Vec<PatchEntry>);
              state.download_size.1 += instruction["FullReplaceSize"].as_u64().unwrap();
            }
            let patch_entry = PatchEntry {
              target_path: file_path,
              delta_path: delta_path,
              has_source: false,
              target_hash: key.clone(),
            };
            state.patch_hashmap.get_mut(&key).unwrap().push(patch_entry); //should we add it to a downloadQueue??
          }
        }
      };
    });
  }

/*
-------------  par ----------------------     -----------------------
| HashQueue |  --> | Check Hash of File | --> | Compare to OldDelta | 
-------------      ----------------------     -----------------------
                                                |                |
                                                |                |
                                        -------------       ----------
                                        | Different |       |  Same  |
                                        -------------       ----------
                                             |                   |
                                             |                   |
                        ----------------------------------   ------------------------------
                        | Add Full File to DownloadQueue |   | Add Delta to DownloadQueue |
                        |      Add size to size sum      |   |    Add size to size sum    |
                        |      Add to Patch HashMap      |   |    Add to Patch Hashmap    |
                        ----------------------------------   ------------------------------
*/
  fn check_hashes(&mut self) {
    let mut hash_queue : Vec<Instruction> = Vec::new();
    {
      //move into new scope so that the mutex does not stay blocked.
      let state = self.state.lock().unwrap();
      hash_queue = state.hash_queue.clone();
    }
    hash_queue.par_iter().for_each(|hash_entry| {
      let file_hash = self.get_hash(&hash_entry.path);
      if hash_entry.old_hash.is_some() && hash_entry.new_hash.is_some() && &file_hash == hash_entry.old_hash.borrow() && &file_hash != hash_entry.new_hash.borrow() {
        //download patch file
        let key = format!("{}_from_{}", hash_entry.new_hash.borrow(), hash_entry.old_hash.borrow());
        let delta_path = format!("{}/{}", self.renegadex_location.borrow(), &key).replace("//","/");
        let mut state = self.state.lock().unwrap();
        if !state.patch_hashmap.contains_key(&key) {
          state.patch_hashmap.insert(key.clone(), Vec::new() as Vec<PatchEntry>);
          state.download_size.1 += hash_entry.delta_size as u64;
        }

        let patch_entry = PatchEntry {
          target_path: hash_entry.path.clone(),
          delta_path: delta_path,
          has_source: true,
          target_hash: key.clone(),
        };
        state.patch_hashmap.get_mut(&key).unwrap().push(patch_entry);
      } else if hash_entry.new_hash.is_some() && &file_hash == hash_entry.new_hash.borrow() {
        //this file is up to date
      } else {
        //this file does not math old hash, nor the new hash, thus it's corrupted
        //download full file
        println!("File {} is corrupted!", &hash_entry.path);
        let key : &String = hash_entry.new_hash.borrow();
        let delta_path = format!("{}/{}", self.renegadex_location.borrow(), &key).replace("//","/");
        let mut state = self.state.lock().unwrap();
        if !state.patch_hashmap.contains_key(key) {
          state.patch_hashmap.insert(key.clone(), Vec::new() as Vec<PatchEntry>);
          state.download_size.1 += hash_entry.full_replace_size as u64;
        }

        let patch_entry = PatchEntry {
          target_path: hash_entry.path.clone(),
          delta_path: delta_path,
          has_source: false,
          target_hash: key.clone(),
        };
        state.patch_hashmap.get_mut(key).unwrap().push(patch_entry);
      }
    });
    self.state.lock().unwrap().finished_hash = true;
  }

/*
 Opens a file and calculates it's SHA256 hash
*/
  fn get_hash(&self, file_path: &String) -> String {
    let mut file = OpenOptions::new().read(true).open(file_path).unwrap();
    let mut sha256 = Sha256::new();
    std::io::copy(&mut file, &mut sha256).unwrap();
    hex::encode_upper(sha256.result())
  }
  
/*
 Spawns magical unicorns
*/
  pub fn poll_progress(&self) {
    let mut state = self.state.clone();
    std::thread::spawn(move || {
      let mut finished_hash = false;
      let mut old_download_size : (u64, u64) = (0, 0);
      while !finished_hash {
        std::thread::sleep(std::time::Duration::from_millis(10));
        let mut download_size : (u64, u64) = (0, 0);
        {
          let mut state = state.lock().unwrap();
          finished_hash = state.finished_hash.clone();
          download_size = state.download_size.clone();
        }
        if old_download_size != download_size {
          println!("{:#?}", download_size);
          old_download_size = download_size;
        }
      }
    });
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn downloader() {
    let mut patcher : Downloader = Downloader::new();
    patcher.set_location("/home/sonny/RenegadeX/game_files/".to_string());
    patcher.retrieve_mirrors(&"https://static.renegade-x.com/launcher_data/version/beta.json".to_string());
    patcher.poll_progress();
    patcher.download();
    assert!(true);
  }
}
