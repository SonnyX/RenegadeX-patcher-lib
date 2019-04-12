extern crate reqwest;
extern crate rayon;
extern crate json;
extern crate sha2;
extern crate ini;
extern crate hex;
extern crate num_cpus;

//Standard library
use std::collections::BTreeMap;
use std::fs::{OpenOptions,DirBuilder};
use std::io::{Read, Write, Seek, SeekFrom};
use std::iter::FromIterator;
use std::ops::Deref;
use std::panic;
use std::sync::{Arc, Mutex};

//Modules
mod mirrors;
pub mod traits;
use mirrors::Mirrors;
use traits::{AsString, BorrowUnwrap, Error};

//External crates
use rayon::prelude::*;
use ini::Ini;
use sha2::{Sha256, Digest};


#[derive(Clone)]
pub struct Progress {
  pub update: Update,
  pub hashes_checked: (u64, u64),
  pub download_size: (u64,u64), //Downloaded .. out of .. bytes
  pub patch_files: (u64, u64), //Patched .. out of .. files
  pub finished_hash: bool,
  pub finished_patching: bool,
}

#[derive(Clone)]
pub enum Update {
  Unknown,
  UpToDate,
  Resume,
  Full,
  Delta,
}

impl Progress {
  fn new() -> Progress {
    Progress {
      update: Update::Unknown,
      hashes_checked: (0,0),
      download_size: (0,0),
      patch_files: (0,0),
      finished_hash: false,
      finished_patching: false,
    }
  }
}

#[derive(Debug)]
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

#[derive(Debug,Clone)]
pub struct PatchEntry {
  target_path: String,
  delta_path: String,
  has_source: bool,
  target_hash: String,
}

#[derive(Debug)]
pub struct DownloadEntry {
  file_path: String,
  file_size: usize,
  file_hash: String,
  patch_entries: Vec<PatchEntry>,
}

pub struct Downloader {
  renegadex_location: Option<String>, //Os dependant
  version_url: Option<String>,
  mirrors: Mirrors,
  instructions: Vec<Instruction>, //instructions.json
  pub state: Arc<Mutex<Progress>>,
  download_hashmap: Mutex<BTreeMap<String, DownloadEntry>>,
  hash_queue: Mutex<Vec<Instruction>>,
  patch_queue: Arc<Mutex<Vec<Vec<PatchEntry>>>>
}


impl Downloader {
  pub fn new() -> Downloader {
    Downloader {
      renegadex_location: None,
      version_url: None,
      mirrors: Mirrors::new(),
      instructions: Vec::new(),
      state: Arc::new(Mutex::new(Progress::new())),
      download_hashmap: Mutex::new(BTreeMap::new()),
      hash_queue: Mutex::new(Vec::new()),
      patch_queue: Arc::new(Mutex::new(Vec::new())),
    }
  }
  pub fn set_location(&mut self, loc: String) {
    self.renegadex_location = Some(format!("{}/", loc).replace("\\","/").replace("//","/"));
  }
  
  pub fn set_version_url(&mut self, url: String) {
    self.version_url = Some(url);
  }

  pub fn retrieve_mirrors(&mut self) -> Result<(), Error> {
    if self.version_url.is_none() {
      return Err(format!("Version URL was not set before calling retrieve_mirrors").into());
    } else {
      return self.mirrors.get_mirrors(self.version_url.borrow());
    }
  }

  pub fn update_available(&self) -> Result<Update, String> {
    if self.mirrors.is_empty() {
      return Err(format!("No mirrors found, aborting! Did you retrieve mirrors?"));
    }
    if self.renegadex_location.is_none() {
      return Err(format!("The RenegadeX location hasn't been set, aborting!"));
    }
    let patch_dir_path = format!("{}/patcher/", self.renegadex_location.borrow()).replace("//", "/");
    match std::fs::read_dir(patch_dir_path) {
      Ok(iter) => {
        if iter.count() != 0 {
          let mut state = self.state.lock().unwrap();
          state.update = Update::Resume;
          return Ok(Update::Resume);
        }
      },
      Err(_e) => {}
    };

    let path = format!("{}UDKGame/Config/DefaultRenegadeX.ini", self.renegadex_location.borrow());
    let conf = match Ini::load_from_file(&path) {
      Ok(file) => file,
      Err(_e) => {
        let mut state = self.state.lock().unwrap();
        state.update = Update::Full;
        return Ok(Update::Full);
      }
    };

    let section = conf.section(Some("RenX_Game.Rx_Game".to_owned())).unwrap();
    let game_version_number = section.get("GameVersionNumber").unwrap();

    if self.mirrors.version_number.borrow() != game_version_number {
      let mut state = self.state.lock().unwrap();
      state.update = Update::Delta;
      return Ok(Update::Delta);
    }
    let mut state = self.state.lock().unwrap();
    state.update = Update::UpToDate;
    return Ok(Update::UpToDate);
  }

  pub fn download(&mut self) -> Result<(), Error> {
    if self.mirrors.is_empty() {
      return Err(format!("No mirrors found! Did you retrieve mirrors?").into());
    }
    if self.instructions.len() == 0 {
      self.retrieve_instructions()?;
    }
    if self.mirrors.mirrors.len() < 3 {
      return Err(format!("Not enough mirrors ({} out of 3) available!", self.mirrors.mirrors.len()).into());
    }
    println!("Retrieved instructions, checking hashes.");
    self.check_hashes();
    let child_process = self.check_patch_queue();
    self.download_files()?;
    child_process.join().unwrap();
    //need to wait somehow for patch_queue to finish.
    let mut state = self.state.lock().unwrap();
    state.update = Update::UpToDate;
    return Ok(());
  }
  
  /*
   * Downloads instructions.json from a mirror, checks its validity and passes it on to process_instructions()
   * -------------------------      ------------  par   ------------------------
   * | retrieve_instructions |  --> | Get Json | ---->  | process_instructions | 
   * -------------------------      ------------        ------------------------
  */
  fn retrieve_instructions(&mut self) -> Result<(), Error> {
    if self.mirrors.is_empty() {
      return Err(format!("No mirrors found! Did you retrieve mirrors?").into());
    }
    let instructions_mutex : Mutex<String> = Mutex::new("".to_string());
    for retry in 0..3 {
      let result : Result<(),Error> = {
        let instructions_url = format!("{}/instructions.json", &self.mirrors.mirrors[retry].address);
        //println!("{}", &instructions_url);
        let text = reqwest::get(&instructions_url)?.text().unwrap();
        // check instructions hash
        let mut sha256 = Sha256::new();
        sha256.input(&text);
        let hash = hex::encode_upper(sha256.result());
        if &hash != self.mirrors.instructions_hash.borrow() {
          Err(format!("Hash of instructions.json ({}) did not match the one specified in release.json ({})!", &hash, self.mirrors.instructions_hash.borrow()).into())
        } else {
          *instructions_mutex.lock().unwrap() = text;
          Ok(())
        }
      };
      if result.is_ok() {
        for _i in 0..retry {
          println!("Removing mirror: {:#?}", &self.mirrors.mirrors[0]);
          self.mirrors.remove(0);
        }
        break;
      } else if result.is_err() && retry == 2 {
        //TODO: This is bound to one day go wrong
        return Err(format!("Couldn't fetch instructions.json").into());
      }
    }
    let instructions_text : String = instructions_mutex.into_inner().unwrap();
    let instructions_data = match json::parse(&instructions_text) {
      Ok(result) => result,
      Err(e) => return Err(format!("Invalid JSON: {}", e).into())
    };
    self.process_instructions(instructions_data);
    return Ok(());
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
    instructions_data.into_inner().par_iter().for_each(|instruction| {
      //lets start off by trying to open the file.
      let file_path = format!("{}{}", self.renegadex_location.borrow(), instruction["Path"].as_string().replace("\\", "/"));
      match OpenOptions::new().read(true).open(&file_path) {
        Ok(_file) => {
          if !instruction["NewHash"].is_null() {
            let mut hash_queue = self.hash_queue.lock().unwrap();
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
            hash_queue.push(hash_entry);
            let mut state = self.state.lock().unwrap();
            state.hashes_checked.1 += 1;
          } else {
            
            //TODO: DeletionQueue, delete it straight away?
          }
        },
        Err(_e) => {
          if !instruction["NewHash"].is_null() {
            let key = instruction["NewHash"].as_string();
            let delta_path = format!("{}patcher/{}", self.renegadex_location.borrow(), &key);
            let mut download_hashmap = self.download_hashmap.lock().unwrap();
            if !download_hashmap.contains_key(&key) {
              let download_entry = DownloadEntry {
                file_path: delta_path.clone(),
                file_size: instruction["FullReplaceSize"].as_usize().unwrap(),
                file_hash: instruction["CompressedHash"].as_string(),
                patch_entries: Vec::new(),
              };
              download_hashmap.insert(key.clone(), download_entry);
              let mut state = self.state.lock().unwrap();
              state.download_size.1 += instruction["FullReplaceSize"].as_u64().unwrap();
            }
            let patch_entry = PatchEntry {
              target_path: file_path,
              delta_path: delta_path,
              has_source: false,
              target_hash: key.clone(),
            };
            let mut state = self.state.lock().unwrap();
            state.patch_files.1 += 1;
            download_hashmap.get_mut(&key).unwrap().patch_entries.push(patch_entry); //should we add it to a downloadQueue??
          }
        }
      };
    });
  }

/*
 * -------------  par ----------------------     -----------------------
 * | HashQueue |  --> | Check Hash of File | --> | Compare to OldDelta | 
 * -------------      ----------------------     -----------------------
 *                                                 |                |
 *                                                 |                |
 *                                         -------------       ----------
 *                                         | Different |       |  Same  |
 *                                         -------------       ----------
 *                                              |                   |
 *                                              |                   |
 *                         ----------------------------------   ------------------------------
 *                         | Add Full File to DownloadQueue |   | Add Delta to DownloadQueue |
 *                         |      Add size to size sum      |   |    Add size to size sum    |
 *                         |      Add to Patch HashMap      |   |    Add to Patch Hashmap    |
 *                         ----------------------------------   ------------------------------
 */
  fn check_hashes(&mut self) {
    let hash_queue = self.hash_queue.lock().unwrap();
    hash_queue.par_iter().for_each(|hash_entry| {
      let file_path_source = format!("{}.vcdiff_src", &hash_entry.path);
      let file_hash = match OpenOptions::new().read(true).open(&file_path_source) {
        Ok(_file) => {
          if hash_entry.old_hash.is_some() && &get_hash(&file_path_source) == hash_entry.old_hash.borrow() {
            match std::fs::remove_file(&hash_entry.path) {
              Ok(()) => {},
              Err(_e) => {
                println!("Couldn't remove file before renaming .vcdiff_src...");
              },
            }
            std::fs::rename(&file_path_source, &hash_entry.path).unwrap();
          } else {
            match std::fs::remove_file(&file_path_source) {
              Ok(()) => {
                println!("Removed .vcdiff_src which did not match old_hash...");
              },
              Err(_e) => {
                println!("Couldn't remove .vcdiff_src which did not match old_hash...");
              }
            }
          }
          get_hash(&hash_entry.path)
        },
        Err(_e) => {
          get_hash(&hash_entry.path)
        },
      };
      if hash_entry.old_hash.is_some() && hash_entry.new_hash.is_some() && &file_hash == hash_entry.old_hash.borrow() && &file_hash != hash_entry.new_hash.borrow() && hash_entry.has_delta {
        //download patch file
        let key = format!("{}_from_{}", hash_entry.new_hash.borrow(), hash_entry.old_hash.borrow());
        let delta_path = format!("{}patcher/{}", self.renegadex_location.borrow(), &key);
        let mut download_hashmap = self.download_hashmap.lock().unwrap();
        if !download_hashmap.contains_key(&key) {
          let download_entry = DownloadEntry {
            file_path: delta_path.clone(),
            file_size: hash_entry.delta_size,
            file_hash: match hash_entry.delta_hash.clone() {
              Some(hash) => hash,
              None => {
                panic!("Delta hash is empty for download_entry: {:?}", hash_entry)
              }
            },
            patch_entries: Vec::new(),
          };
          download_hashmap.insert(key.clone(), download_entry);
          let mut state = self.state.lock().unwrap();
          state.download_size.1 += hash_entry.delta_size as u64;
        }

        let patch_entry = PatchEntry {
          target_path: hash_entry.path.clone(),
          delta_path: delta_path,
          has_source: true,
          target_hash: hash_entry.new_hash.clone().unwrap(),
        };
        let mut state = self.state.lock().unwrap();
        state.patch_files.1 += 1;
        download_hashmap.get_mut(&key).unwrap().patch_entries.push(patch_entry);
      } else if hash_entry.new_hash.is_some() && &file_hash == hash_entry.new_hash.borrow() {
        //this file is up to date
      } else {
        //this file does not math old hash, nor the new hash, thus it's corrupted
        //download full file
        println!("No suitable patch file found for \"{}\", downloading full file!", &hash_entry.path);
        let key : &String = hash_entry.new_hash.borrow();
        let delta_path = format!("{}patcher/{}", self.renegadex_location.borrow(), &key);
        let mut download_hashmap = self.download_hashmap.lock().unwrap();
        if !download_hashmap.contains_key(key) {
         let download_entry = DownloadEntry {
            file_path: delta_path.clone(),
            file_size: hash_entry.full_replace_size,
            file_hash: match hash_entry.compressed_hash.clone() {
              Some(hash) => hash,
              None => {
                panic!("Delta hash is empty for download_entry: {:?}", hash_entry)
              }
            },
            patch_entries: Vec::new(),
          };
          download_hashmap.insert(key.clone(), download_entry);
          let mut state = self.state.lock().unwrap();
          state.download_size.1 += hash_entry.full_replace_size as u64;
        }

        let patch_entry = PatchEntry {
          target_path: hash_entry.path.clone(),
          delta_path: delta_path,
          has_source: false,
          target_hash: hash_entry.new_hash.clone().unwrap(),
        };
        let mut state = self.state.lock().unwrap();
        state.patch_files.1 += 1;
        download_hashmap.get_mut(key).unwrap().patch_entries.push(patch_entry);
      }
      let mut state = self.state.lock().unwrap();
      state.hashes_checked.0 += 1;
    });
    self.state.lock().unwrap().finished_hash = true;
  }


/*
 * Iterates over the download_hashmap and calls download_and_patch for each DownloadEntry.
 */
  fn download_files(&self) -> Result<(), Error> {
    let dir_path = format!("{}patcher/", self.renegadex_location.borrow());
    DirBuilder::new().recursive(true).create(dir_path).unwrap();
    let download_hashmap = self.download_hashmap.lock().unwrap();
    let mut sorted_downloads_by_size = Vec::from_iter(download_hashmap.deref());
    sorted_downloads_by_size.sort_by(|&(_, a), &(_,b)| b.file_size.cmp(&a.file_size));
    let num_threads = num_cpus::get()*3;
    let pool = rayon::ThreadPoolBuilder::new().num_threads(num_threads).build().unwrap();
    pool.install(|| -> Result<(), Error> {
      sorted_downloads_by_size.par_iter().try_for_each(
        |(key, download_entry)| self.download_and_patch(key, download_entry)
      )
    })?;
    return Ok(());
  }

  fn download_and_patch(&self, key: &String, download_entry: &DownloadEntry) -> Result<(), Error> {
    for attempt in 0..5 {
      //TODO add in a random number generator in order to balance the load between the mirrors
      let mirror = self.mirrors.get_mirror();
      let download_url = match download_entry.patch_entries[0].has_source {
        true => format!("{}/delta/{}", &mirror, &key),
        false => format!("{}/full/{}", &mirror, &key)
      };
      match self.download_file(&download_url, download_entry, if attempt == 0 { true } else { false }) {
        Ok(()) => {
          break
        },
        Err(e) => {
          if attempt == 4 { return Err(format!("Couldn't download file: {}", &key).into()) }
          else { println!("Downloading file from {} failed due to error: {}", download_url, e); }
        }
      };
    }
    //apply delta
    let mut patch_queue = self.patch_queue.lock().unwrap();
    patch_queue.push(download_entry.patch_entries.clone());
    return Ok(())
  }

  fn check_patch_queue(&self) -> std::thread::JoinHandle<()> {
    let state = self.state.clone();
    let patch_queue_unlocked = self.patch_queue.clone();
    let renegadex_location = self.renegadex_location.clone();
    let num_threads = num_cpus::get()-1;
    std::thread::spawn(move || {
      let pool = rayon::ThreadPoolBuilder::new().num_threads(num_threads).build().unwrap();
      pool.install(|| {
        rayon::scope(|s| {
          for _i in 0..num_threads {
            s.spawn(|_| {
              let mut patch_files = state.lock().unwrap().patch_files.clone();
              while patch_files.0 != patch_files.1 {
                // Check for entry in patch_queue, get one, remove it, free the mutex, process entry.
                let patch_entries : Option<Vec<PatchEntry>>;
                {
                  let mut patch_queue = patch_queue_unlocked.lock().unwrap();
                  patch_entries = patch_queue.pop();
                }
                if patch_entries.is_some() {
                  patch_entries.borrow().par_iter().for_each(|patch_entry| {
                    println!("Patching with diff file: {}", &patch_entry.delta_path);
                    apply_patch(patch_entry, state.clone()).unwrap();
                    println!("Patching success: {}", &patch_entry.delta_path);
                  });
                  std::fs::remove_file(patch_entries.borrow().first().unwrap().delta_path.clone()).unwrap();
                  patch_files = state.lock().unwrap().patch_files.clone();
                } else {
                  std::thread::sleep(std::time::Duration::from_millis(20));
                  patch_files = state.lock().unwrap().patch_files.clone();
                }
              }
            });
          }
        });
        {
          let mut state = state.lock().unwrap();
          state.finished_patching = true;
        }
        //remove patcher folder and all remaining files in there:
        std::fs::remove_dir_all(format!("{}patcher/", renegadex_location.unwrap())).unwrap();
      });
    })
  }


/*
 * Downloads the file in parts
 */
  fn download_file(&self, download_url: &String, download_entry: &DownloadEntry, first_attempt: bool) -> Result<(), Error> {
    let part_size = 10u64.pow(6) as usize; //1.000.000
    let mut f = match OpenOptions::new().read(true).write(true).create(true).open(&download_entry.file_path) {
      Ok(file) => file,
      Err(e) => {
        return Err(format!("Couldn't open delta_file \"{}\": {:?}", &download_entry.file_path, e).into());
      }
    };
    //set the size of the file, add a 32bit integer to the end of the file as a means of tracking progress. We won't download parts async.
    let parts_amount : usize = download_entry.file_size / part_size + if download_entry.file_size % part_size > 0 {1} else {0};
    let file_size : usize = download_entry.file_size + 4;
    if (f.metadata().unwrap().len() as usize) < file_size {
      if f.metadata().unwrap().len() == (download_entry.file_size as u64) {
        //If hash is correct, return.
        //Otherwise download again.
        let hash = get_hash(&download_entry.file_path);
        if &hash == &download_entry.file_hash {
          let mut state = self.state.lock().unwrap();
          state.download_size.0 += (download_entry.file_size) as u64;
          return Ok(());
        }
      }
      match f.set_len(file_size as u64) {
        Ok(()) => {},
        Err(e) => {
          return Err(format!("Could not change file size of patch file, is it in use?\n{}",e).into());
        }
      }
    }
    let http_client = reqwest::Client::new();
    f.seek(SeekFrom::Start((download_entry.file_size) as u64)).unwrap();
    let mut buf = [0,0,0,0];
    f.read_exact(&mut buf).unwrap();
    let resume_part : usize = u32::from_be_bytes(buf) as usize;
    if resume_part != 0 { 
      println!("Resuming download \"{}\" from part {} out of {}", &download_entry.file_hash, resume_part, parts_amount);
      if first_attempt {
        let mut state = self.state.lock().unwrap();
        state.download_size.0 += (part_size * resume_part) as u64;
      }
    };
    //iterate over all parts, downloading them into memory, writing them into the file, adding one to the counter at the end of the file.
    for part_int in resume_part..parts_amount {
      let bytes_start = part_int * part_size;
      let mut bytes_end = part_int * part_size + part_size -1;
      if bytes_end > download_entry.file_size {
        bytes_end = download_entry.file_size.clone();
      }
      let download_request = http_client.get(download_url).header(reqwest::header::RANGE,format!("bytes={}-{}", bytes_start, bytes_end));
      let download_response = download_request.send();
      f.seek(SeekFrom::Start(bytes_start as u64)).unwrap();
      let mut content : Vec<u8> = Vec::with_capacity(bytes_end - bytes_start + 1);
      download_response?.read_to_end(&mut content).unwrap();
      f.write_all(&content).unwrap();
      //completed downloading and writing this part, so update the progress-tracker at the end of the file
      f.seek(SeekFrom::Start((download_entry.file_size) as u64)).unwrap();
      f.write_all(&(part_int as u32).to_be_bytes()).unwrap();
      let mut state = self.state.lock().unwrap();
      state.download_size.0 += (bytes_end - bytes_start) as u64;
    }
    //Remove the counter at the end of the file to finish the vcdiff file
    f.set_len(download_entry.file_size as u64).unwrap();
    
    //Let's make sure the downloaded file matches the Hash found in Instructions.json
    let hash = get_hash(&download_entry.file_path);
    if &hash != &download_entry.file_hash {
      return Err(format!("File \"{}\"'s hash ({}) did not match with the one provided in Instructions.json ({})", &download_entry.file_path, &hash, &download_entry.file_hash).into());
    }
    return Ok(());
  }
  
/*
 * Spawns magical unicorns, only usefull for testing
 */
  pub fn poll_progress(&self) {
    let state = self.state.clone();
    std::thread::spawn(move || {
      let mut finished_hash : bool;
      let mut finished_patching = false;
      let mut old_time = std::time::Instant::now();
      let mut old_download_size : (u64, u64) = (0, 0);
      let mut old_patch_files : (u64, u64) = (0, 0);
      let mut old_hashes_checked : (u64, u64) = (0, 0);
      while !finished_patching {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let state = state.lock().unwrap();
        finished_hash = state.finished_hash.clone();
        finished_patching = state.finished_patching.clone();
        let download_size : (u64, u64) = state.download_size.clone();
        let patch_files : (u64, u64) = state.patch_files.clone();
        let hashes_checked : (u64, u64) = state.hashes_checked.clone();
        let elapsed = old_time.elapsed();
        old_time = std::time::Instant::now();
        if !finished_hash {
          if old_download_size != download_size {
            println!("Comparing files, total to be downloaded: {:.1} MB", (download_size.1 as f64)*0.000001);
          }
          if old_hashes_checked != hashes_checked {
            println!("Checked {} out of {} hashes.", hashes_checked.0, hashes_checked.1);
          }
        } else {
          if old_download_size != download_size {
            println!("Downloaded {:.1}/{:.1} MB, speed: {:.3} MB/s", (download_size.0 as f64)*0.000001, (download_size.1 as f64)*0.000001, ((download_size.0 - old_download_size.0) as f64)/(elapsed.as_micros() as f64));
          }
          if patch_files != old_patch_files {
            println!("Patched {}/{} files", patch_files.0, patch_files.1);
          }
        }
        old_download_size = download_size;
        old_patch_files = patch_files;
        old_hashes_checked = hashes_checked;
      }
    });
  }

  pub fn get_progress(&self) -> Arc<Mutex<Progress>> {
    self.state.clone()
  }
}

/*
 * Applies the vcdiff patch file to the target file.
 * 
 * -------------- par --------------------------------------------------
 * | DeltaQueue | --> | apply patch to all files that match this Delta |
 * --------------     --------------------------------------------------
 */
  fn apply_patch(patch_entry: &PatchEntry, state: Arc<Mutex<Progress>>) -> Result<(), Error> {
    let mut dir_path = patch_entry.target_path.clone();
    dir_path.truncate(patch_entry.target_path.rfind('/').unwrap());
    DirBuilder::new().recursive(true).create(dir_path).unwrap();
    if patch_entry.has_source {
      let source_path = format!("{}.vcdiff_src", &patch_entry.target_path);
      std::fs::rename(&patch_entry.target_path, &source_path).unwrap();
      xdelta::decode_file(Some(&source_path), &patch_entry.delta_path, &patch_entry.target_path);
      std::fs::remove_file(&source_path).unwrap();
    } else {
      //there is supposed to be no source file, so make sure it doesn't exist either!
      match std::fs::remove_file(&patch_entry.target_path) {
        Ok(()) => (),
        Err(_e) => ()
      };
      xdelta::decode_file(None, &patch_entry.delta_path, &patch_entry.target_path);
    }
    let hash = get_hash(&patch_entry.target_path);
    if &hash != &patch_entry.target_hash {
      return Err(format!("Hash for file {} is incorrect!\nGot hash: {}\nExpected hash: {}", &patch_entry.target_path, &hash, &patch_entry.target_hash).into());
    }
    let mut state = state.lock().unwrap();
    state.patch_files.0 += 1;
    return Ok(());
  }


/*
 * Opens a file and calculates it's SHA256 hash
 */
  fn get_hash(file_path: &String) -> String {
    let mut file = OpenOptions::new().read(true).open(file_path).unwrap();
    let mut sha256 = Sha256::new();
    std::io::copy(&mut file, &mut sha256).unwrap();
    hex::encode_upper(sha256.result())
  }

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn downloader() {
    let mut patcher : Downloader = Downloader::new();
    patcher.set_location("/home/sonny/RenegadeX/game_files/".to_string());
    patcher.set_version_url("https://static.renegade-x.com/launcher_data/version/release.json".to_string());
    patcher.retrieve_mirrors().unwrap();
    match patcher.update_available().unwrap() {
      Update::UpToDate => {
        println!("Game up to date!");
        patcher.poll_progress();
        patcher.download().unwrap();
      },
      Update::Resume | Update::Delta | Update::Full | Update::Unknown => {
        println!("Update available!");
        patcher.poll_progress();
        patcher.download().unwrap();
      }
    };
    assert!(true);
  }
}
