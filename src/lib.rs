extern crate reqwest;
extern crate rayon;
extern crate json;
extern crate sha2;
extern crate ini;
extern crate hex;
extern crate num_cpus;
extern crate hyper;
#[macro_use] extern crate futures;
extern crate tokio;
extern crate url;
extern crate tokio_reactor;

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
mod downloader;
pub mod traits;
use downloader::BufWriter;
use mirrors::{Mirrors, Mirror};
use traits::{AsString, BorrowUnwrap, Error};

//External crates
use rayon::prelude::*;
use ini::Ini;
use sha2::{Sha256, Digest};
use hyper::rt::Future;

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
struct Directory {
  name: std::ffi::OsString,
  subdirectories: Vec<Directory>,
  files: Vec<std::path::PathBuf>,
}

impl Directory {
  pub fn get_or_create_subdirectory(&mut self, name: std::ffi::OsString) -> &mut Directory {
    for index in 0..self.subdirectories.len() {
      if self.subdirectories[index].name == name {
        return &mut self.subdirectories[index];
      }
    }
    self.subdirectories.push(
      Directory {
        name: name,
        subdirectories: Vec::new(),
        files: Vec::new(), 
      }
    );
    return self.subdirectories.last_mut().unwrap();
  }

  pub fn get_subdirectory(&self, name: std::ffi::OsString) -> Option<&Directory> {
    for index in 0..self.subdirectories.len() {
      if self.subdirectories[index].name == name {
        return Some(&self.subdirectories[index]);
      }
    }
    return None;
  }

  pub fn directory_exists(&self, path: std::path::PathBuf) -> bool {
    //split up path into an iter and push it to temporary path's, if it's all done then we're good
    let mut temp = self;
    for directory in path.iter() {
      temp = match temp.get_subdirectory(directory.to_owned()) {
        Some(subdir) => subdir,
        None => {
          return false;
        },
      };
    }
    return true;
  }

  pub fn file_exists(&self, file: std::path::PathBuf) -> bool {
    //split up path into an iter and push it to temporary path's, if it's all done then we're good
    if file.file_name().unwrap() == "InstallInfo.xml" {
      return true;
    }
    let mut temp = self;
    let mut dir = file.clone();
    dir.pop();
    for directory in dir.iter() {
      temp = match temp.get_subdirectory(directory.to_owned()) {
        Some(subdir) => subdir,
        None => {
          return false;
        },
      };
    }
    return temp.files.contains(&file);
  }

}

#[derive(Debug, Clone)]
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

impl Default for Downloader {
  fn default() -> Self {
    Self::new()
  }
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

  ///
  ///
  ///
  ///
  pub fn set_location(&mut self, loc: String) {
    self.renegadex_location = Some(format!("{}/", loc).replace("\\","/").replace("//","/"));
  }
  
  ///
  ///
  ///
  ///
  pub fn set_version_url(&mut self, url: String) {
    self.version_url = Some(url);
  }

  ///
  ///
  ///
  ///
  pub fn retrieve_mirrors(&mut self) -> Result<(), Error> {
    if self.version_url.is_none() {
      Err("Version URL was not set before calling retrieve_mirrors".to_string().into())
    } else {
      self.mirrors.get_mirrors(self.version_url.borrow())
    }
  }

  ///
  ///
  ///
  ///
  pub fn update_available(&self) -> Result<Update, String> {
    if self.mirrors.is_empty() {
      return Err("No mirrors found, aborting! Did you retrieve mirrors?".to_string());
    }
    if self.renegadex_location.is_none() {
      return Err("The RenegadeX location hasn't been set, aborting!".to_string());
    }
    let patch_dir_path = format!("{}/patcher/", self.renegadex_location.borrow()).replace("//", "/");
    match std::fs::read_dir(patch_dir_path) {
      Ok(iter) => {
        if iter.count() != 0 {
          let mut state = self.state.lock().unwrap();
          state.update = Update::Resume;
          drop(state);
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
        drop(state);
        return Ok(Update::Full);
      }
    };

    let section = conf.section(Some("RenX_Game.Rx_Game".to_owned())).unwrap();
    let game_version_number = section.get("GameVersionNumber").unwrap();

    if self.mirrors.version_number.borrow() != game_version_number {
      let mut state = self.state.lock().unwrap();
      state.update = Update::Delta;
      drop(state);
      return Ok(Update::Delta);
    }
    let mut state = self.state.lock().unwrap();
    state.update = Update::UpToDate;
    drop(state);
    Ok(Update::UpToDate)
  }

  ///
  ///
  ///
  ///
  pub fn download(&mut self) -> Result<(), Error> {
    let mut progress = self.state.lock().unwrap();
    progress.update = Update::Unknown;
    progress.hashes_checked = (0,0);
    progress.download_size = (0,0);
    progress.patch_files = (0,0);
    progress.finished_hash = false;
    progress.finished_patching = false;
    drop(progress);
    self.download_hashmap = Mutex::new(BTreeMap::new());
    self.hash_queue = Mutex::new(Vec::new());
    self.patch_queue = Arc::new(Mutex::new(Vec::new()));

    if self.instructions.is_empty() {
      self.retrieve_instructions()?;
    }
    self.process_instructions();
    println!("Retrieved instructions, checking hashes.");
    self.check_hashes();
    let child_process = self.check_patch_queue();
    self.download_files()?;
    child_process.join().unwrap();
    //need to wait somehow for patch_queue to finish.
    let mut state = self.state.lock().unwrap();
    state.update = Update::UpToDate;
    drop(state);
    Ok(())
  }
  
  /*
   * Downloads instructions.json from a mirror, checks its validity and passes it on to process_instructions()
   * -------------------------      ------------
   * | retrieve_instructions |  --> | Get Json |
   * -------------------------      ------------
  */
  fn retrieve_instructions(&mut self) -> Result<(), Error> {
    if self.mirrors.is_empty() {
      return Err("No mirrors found! Did you retrieve mirrors?".to_string().into());
    }
    if !self.instructions.is_empty() {
      return Ok(());
    }
    let instructions_mutex : Mutex<String> = Mutex::new("".to_string());
    for retry in 0..3 {
      let mirror = self.mirrors.get_mirror();
      let result : Result<(),Error> = {
        let instructions_url = format!("{}/instructions.json", &mirror.address);
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
        break;
      } else if result.is_err() && retry == 2 {
        //TODO: This is bound to one day go wrong
        return Err("Couldn't fetch instructions.json".to_string().into());
      } else {
        println!("Removing mirror: {:#?}", &mirror);
        self.mirrors.remove(mirror);
      }
    }
    let instructions_text : String = instructions_mutex.into_inner().unwrap();
    let instructions_data = match json::parse(&instructions_text) {
      Ok(result) => result,
      Err(e) => return Err(format!("Invalid JSON: {}", e).into())
    };
    instructions_data.into_inner().iter().for_each(|instruction| {
      let file_path = format!("{}{}", self.renegadex_location.borrow(), instruction["Path"].as_string().replace("\\", "/"));
      self.instructions.push(Instruction {
              path:                file_path,
              old_hash:            instruction["OldHash"].as_string_option(),
              new_hash:            instruction["NewHash"].as_string_option(),
              compressed_hash:     instruction["CompressedHash"].as_string_option(),
              delta_hash:          instruction["DeltaHash"].as_string_option(),
              full_replace_size:   instruction["FullReplaceSize"].as_usize().unwrap(),
              delta_size:          instruction["DeltaSize"].as_usize().unwrap(),
              has_delta:           instruction["HasDelta"].as_bool().unwrap()
            });
    });
    Ok(())
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
  fn process_instructions(&self) {
    self.instructions.par_iter().for_each(|instruction| {
      //lets start off by trying to open the file.
      match OpenOptions::new().read(true).open(&instruction.path) {
        Ok(_file) => {
          if instruction.new_hash.is_some() {
            let mut hash_queue = self.hash_queue.lock().unwrap();
            hash_queue.push(instruction.clone());
            drop(hash_queue);
            let mut state = self.state.lock().unwrap();
            state.hashes_checked.1 += 1;
            drop(state);
          } else {
            println!("Found entry {} that needs deleting.", instruction.path);
            //TODO: DeletionQueue, delete it straight away?
          }
        },
        Err(_e) => {
          if let Some(key) = &instruction.new_hash {
            let delta_path = format!("{}patcher/{}", self.renegadex_location.borrow(), &key);
            let mut download_hashmap = self.download_hashmap.lock().unwrap();
            if !download_hashmap.contains_key(key) {
              let download_entry = DownloadEntry {
                file_path: delta_path.clone(),
                file_size: instruction.full_replace_size,
                file_hash: instruction.compressed_hash.clone().unwrap(),
                patch_entries: Vec::new(),
              };
              download_hashmap.insert(key.clone(), download_entry);
              let mut state = self.state.lock().unwrap();
              state.download_size.1 += instruction.full_replace_size as u64;
              drop(state);
            }
            let patch_entry = PatchEntry {
              target_path: instruction.path.clone(),
              delta_path,
              has_source: false,
              target_hash: key.clone(),
            };
            download_hashmap.get_mut(key).unwrap().patch_entries.push(patch_entry); //should we add it to a downloadQueue??
            drop(download_hashmap);
            let mut state = self.state.lock().unwrap();
            state.patch_files.1 += 1;
            drop(state);
          }
        }
      };
    });
  }

  pub fn remove_unversioned(&mut self) -> Result<(), Error> {
    if self.instructions.is_empty() {
      self.retrieve_instructions()?;
    }
    let mut versioned_files = Directory {
      name: "".into(),
      subdirectories: Vec::new(),
      files: Vec::new(),
    };
    let renegadex_path = std::path::PathBuf::from(self.renegadex_location.borrow());
    for entry in self.instructions.iter() {
      let mut path = &mut versioned_files;
      let mut directory_iter = std::path::PathBuf::from(&entry.path).strip_prefix(&renegadex_path).unwrap().to_path_buf();
      directory_iter.pop();
      for directory in directory_iter.iter() {
        path = path.get_or_create_subdirectory(directory.to_owned());
      }
      //path should be the correct directory now.
      //thus add file to path.files
      path.files.push(std::path::PathBuf::from(&entry.path).strip_prefix(&renegadex_path).unwrap().to_path_buf());
    }
    let files = std::fs::read_dir(&self.renegadex_location.borrow()).unwrap();
    for file in files {
      let file = file.unwrap();
      if file.file_type().unwrap().is_dir() {
        if versioned_files.directory_exists(file.path().strip_prefix(&renegadex_path).unwrap().to_owned()) {
          self.read_dir(&file.path(), &versioned_files, &renegadex_path)?;
        } else {
          println!("Remove directory: {:?}", &file.path());
        }
      } else {
        println!("Remove file: {:?}", &file.path());
        //doubt antything
      }
    }
    Ok(())
  }

  fn read_dir(&self, dir: &std::path::Path, versioned_files: &Directory, renegadex_path: &std::path::PathBuf) -> Result<(),Error> {
    let files = std::fs::read_dir(dir).unwrap();
    for file in files {
      let file = file.unwrap();
      if file.file_type().unwrap().is_dir() {
        if versioned_files.directory_exists(file.path().strip_prefix(&renegadex_path).unwrap().to_owned()) {
          self.read_dir(&file.path(), versioned_files, renegadex_path)?;
        } else {
          std::fs::remove_dir_all(&file.path())?;
        }
      } else {
        if !versioned_files.file_exists(file.path().strip_prefix(&renegadex_path).unwrap().to_owned()) {
          std::fs::remove_file(&file.path())?;
        }
        //doubt antything
      }
    }
    Ok(())
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
          drop(state);
        }

        let patch_entry = PatchEntry {
          target_path: hash_entry.path.clone(),
          delta_path,
          has_source: true,
          target_hash: hash_entry.new_hash.clone().unwrap(),
        };
        download_hashmap.get_mut(&key).unwrap().patch_entries.push(patch_entry);
        drop(download_hashmap);
        let mut state = self.state.lock().unwrap();
        state.patch_files.1 += 1;
        state.hashes_checked.0 += 1;
        drop(state);
      } else if hash_entry.new_hash.is_some() && &file_hash == hash_entry.new_hash.borrow() {
        //this file is up to date
        let mut state = self.state.lock().unwrap();
        state.hashes_checked.0 += 1;
        drop(state);
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
          drop(state);
        }

        let patch_entry = PatchEntry {
          target_path: hash_entry.path.clone(),
          delta_path,
          has_source: false,
          target_hash: hash_entry.new_hash.clone().unwrap(),
        };
        download_hashmap.get_mut(key).unwrap().patch_entries.push(patch_entry);
        drop(download_hashmap);
        let mut state = self.state.lock().unwrap();
        state.patch_files.1 += 1;
        state.hashes_checked.0 += 1;
        drop(state);
      }
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
    let pool = rayon::ThreadPoolBuilder::new().breadth_first().num_threads(20).build().unwrap();
    pool.install(|| {
      rayon::scope(|s| {
        for (key, download_entry) in sorted_downloads_by_size.into_iter() {
          s.spawn(move |_| {self.download_and_patch(key, download_entry).unwrap();});
        }
      })
    });
    Ok(())
  }

  ///
  ///
  ///
  ///
  fn download_and_patch(&self, key: &str, download_entry: &DownloadEntry) -> Result<(), Error> {
    for attempt in 0..5 {
      //TODO add in a random number generator in order to balance the load between the mirrors
      let mirror = self.mirrors.get_mirror();
      let download_url = match download_entry.patch_entries[0].has_source {
        true => format!("{}/delta/{}", &mirror.address, &key),
        false => format!("{}/full/{}", &mirror.address, &key)
      };
      match self.download_file(&mirror, &download_url, download_entry, attempt == 0) {
        Ok(()) => {
          break
        },
        Err(e) => {
          println!("Download {} failed with error message: {}", &download_url, e);
          if attempt == 4 { return Err(format!("Couldn't download file: {}", &key).into()) }
          else {
            println!("Downloading file from {} failed due to error: {}", download_url, e);
            if e.remove_mirror {
              println!("Removing mirror: {}", mirror.address);
              self.mirrors.remove(mirror);
            }
          }
        }
      };
    }
    //apply delta
    let mut patch_queue = self.patch_queue.lock().unwrap();
    patch_queue.push(download_entry.patch_entries.clone());
    drop(patch_queue);
    Ok(())
  }

  ///
  ///
  ///
  ///
  fn check_patch_queue(&self) -> std::thread::JoinHandle<()> {
    let unlocked_state = self.state.clone();
    let patch_queue_unlocked = self.patch_queue.clone();
    let renegadex_location = self.renegadex_location.clone();
    let num_threads = num_cpus::get()-1;
    std::thread::spawn(move || {
      let pool = rayon::ThreadPoolBuilder::new().num_threads(num_threads).build().unwrap();
      pool.install(|| {
        rayon::scope(|s| {
          for _i in 0..num_threads {
            s.spawn(|_| {
              let state = unlocked_state.lock().unwrap();
              let mut patch_files = state.patch_files;
              drop(state);
              while patch_files.0 != patch_files.1 {
                // Check for entry in patch_queue, get one, remove it, free the mutex, process entry.
                let patch_entries : Option<Vec<PatchEntry>>;
                {
                  let mut patch_queue = patch_queue_unlocked.lock().unwrap();
                  patch_entries = patch_queue.pop();
                  drop(patch_queue);
                }
                if patch_entries.is_some() {
                  patch_entries.borrow().par_iter().for_each(|patch_entry| {
                    //println!("Patching with diff file: {}", &patch_entry.delta_path);
                    apply_patch(patch_entry, unlocked_state.clone()).unwrap();
                    //println!("Patching success: {}", &patch_entry.delta_path);
                  });
                  std::fs::remove_file(patch_entries.borrow().first().unwrap().delta_path.clone()).unwrap();
                  let state = unlocked_state.lock().unwrap();
                  patch_files = state.patch_files;
                  drop(state);
                } else {
                  std::thread::sleep(std::time::Duration::from_millis(20));
                  let state = unlocked_state.lock().unwrap();
                  patch_files = state.patch_files;
                  drop(state);
                }
              }
            });
          }
        });
        {
          let mut state = unlocked_state.lock().unwrap();
          state.finished_patching = true;
          drop(state);
        }
        //remove patcher folder and all remaining files in there:
        std::fs::remove_dir_all(format!("{}patcher/", renegadex_location.unwrap())).unwrap();
      });
    })
  }

  ///
  /// Downloads the file in parts
  ///
  ///
  fn download_file(&self, mirror: &Mirror, download_url: &str, download_entry: &DownloadEntry, first_attempt: bool) -> Result<(), Error> {
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
        if hash == download_entry.file_hash {
          let mut state = self.state.lock().unwrap();
          state.download_size.0 += (download_entry.file_size) as u64;
          drop(state);
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
    //We have set up the file
    f.seek(SeekFrom::Start(download_entry.file_size as u64)).unwrap();
    let mut buf = [0,0,0,0];
    f.read_exact(&mut buf).unwrap();
    let resume_part : usize = u32::from_be_bytes(buf) as usize;
    if resume_part != 0 { 
      println!("Resuming download \"{}\" from part {} out of {}", &download_entry.file_path, resume_part, parts_amount);
      if first_attempt {
        let mut state = self.state.lock().unwrap();
        state.download_size.0 += (part_size * resume_part) as u64;
        drop(state);
      }
    };

    let mut future;
    {
      let unlocked_state = self.state.clone();
      let mut writer = BufWriter::new(f.try_clone().unwrap(), move | writer, total_written | {
        //When the buffer is being written to file, this closure gets executed
        let parts = *total_written / part_size as u64;
        writer.seek(SeekFrom::End(-4)).unwrap();
        writer.write_all(&(parts as u32).to_be_bytes()).unwrap();
        writer.seek(SeekFrom::Start(*total_written)).unwrap();
      });
      writer.seek(SeekFrom::Start((part_size * resume_part) as u64)).unwrap();

      let url = download_url.parse::<hyper::Uri>().unwrap();
      let trunc_size = download_entry.file_size as u64;
      future = tokio::net::TcpStream::from_std(std::net::TcpStream::connect(&mirror.ip).unwrap(), &tokio_reactor::Handle::default()).map(|tcp| {
        hyper::client::conn::handshake(tcp)
      }).unwrap().and_then(move |(mut client, conn)| {
        let mut req = hyper::Request::builder();
        req.uri(url.path()).header("host", url.host().unwrap()).header("User-Agent", "sonny-launcher/1.0");
        if resume_part != 0 {
          req.header("Range", format!("bytes={}-{}", (part_size * resume_part), download_entry.file_size));
        };
        let req = req.body(hyper::Body::empty()).unwrap();
        let res = client.send_request(req).and_then(move |res| process_response(res, writer, unlocked_state, f, trunc_size));
        // Put in an Option so poll_fn can return it later
        let mut conn = Some(conn);
        let until_upgrade = futures::future::poll_fn(move || {
          try_ready!(conn.as_mut().unwrap().poll_without_shutdown());
          Ok(futures::Async::Ready(conn.take().unwrap()))
        });

        res.join(until_upgrade)
      });
    }
    tokio::runtime::current_thread::Runtime::new().unwrap().block_on(future)?;

    //Let's make sure the downloaded file matches the Hash found in Instructions.json
    let hash = get_hash(&download_entry.file_path);
    if hash != download_entry.file_hash {
      let mut state = self.state.lock().unwrap();
      state.download_size.0 -= download_entry.file_size as u64;
      drop(state);
      return Err(format!("File \"{}\"'s hash ({}) did not match with the one provided in Instructions.json ({})", &download_entry.file_path, &hash, &download_entry.file_hash).into());
    }
    Ok(())
  }

  ///
  /// Spawns magical unicorns, only usefull for testing
  ///
  ///
  pub fn poll_progress(&self) {
    let state = self.state.clone();
    std::thread::spawn(move || {
      let mut finished_hash : bool;
      let mut finished_patching = false;
      let mut old_download_size : (u64, u64) = (0, 0);
      let mut old_patch_files : (u64, u64) = (0, 0);
      let mut old_hashes_checked : (u64, u64) = (0, 0);
      while !finished_patching {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        let state = state.lock().unwrap();
        finished_hash = state.finished_hash;
        finished_patching = state.finished_patching;
        let download_size : (u64, u64) = state.download_size;
        let patch_files : (u64, u64) = state.patch_files;
        let hashes_checked : (u64, u64) = state.hashes_checked;
        drop(state);
        if !finished_hash {
          if old_download_size != download_size {
            println!("Comparing files, total to be downloaded: {:.1} MB", (download_size.1 as f64)*0.000_001);
          }
          if old_hashes_checked != hashes_checked {
            println!("Checked {} out of {} hashes.", hashes_checked.0, hashes_checked.1);
          }
        } else {
          if old_download_size != download_size {
            println!("Downloaded {:.1}/{:.1} MB, speed: {}/s", (download_size.0 as f64)*0.000_001, (download_size.1 as f64)*0.000_001, convert((download_size.0 - old_download_size.0) as f64));
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

  ///
  ///
  ///
  ///
  pub fn get_progress(&self) -> Arc<Mutex<Progress>> {
    self.state.clone()
  }
}

fn process_response<W: Write, F: FnMut(&mut W, &mut u64)>(res: hyper::Response<hyper::Body>, mut writer: downloader::BufWriter<W, F>, unlocked_state: Arc<Mutex<Progress>>, f: std::fs::File, trunc_size: u64) 
-> impl Future<Item = Result<(), traits::Error>, Error = hyper::Error> {
  use hyper::rt::*;
  let abort_in_error = res.status() != 200 && res.status() != 206;

  res.into_body().for_each(move |chunk| {
    if !abort_in_error {
      let ret = writer.write_all(&chunk).map_err(|e| panic!("Writer encountered an error: {}", e));
      let mut state = unlocked_state.lock().unwrap();
      state.download_size.0 += chunk.len() as u64;
      drop(state);
      ret
    } else {
      let mut vec = Vec::new();
      let ret = vec.write_all(&chunk).map_err(|e| panic!("Writer encountered an error: {}", e));
      ret
    }
  }).and_then(move |_| {
    if !abort_in_error {
      f.set_len(trunc_size).unwrap();
      drop(f);
      Ok(Ok(()))
    } else {
      Ok(Err("Unexpected response: found status code 200 or 206!".into()))
    }
  })
}

pub fn convert(num: f64) -> String {
  let negative = if num.is_sign_positive() { "" } else { "-" };
  let num = num.abs();
  let units = ["B", "kB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
  if num < 1_f64 {
    return format!("{}{} {}", negative, num, "B");
  }
  let delimiter = 1000_f64;
  let exponent = std::cmp::min((num.ln() / delimiter.ln()).floor() as i32, (units.len() - 1) as i32);
  let pretty_bytes = format!("{:.2}", num / delimiter.powi(exponent)).parse::<f64>().unwrap() * 1_f64;
  let unit = units[exponent as usize];
  format!("{}{} {}", negative, pretty_bytes, unit)
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
  if hash != patch_entry.target_hash {
    return Err(format!("Hash for file {} is incorrect!\nGot hash: {}\nExpected hash: {}", &patch_entry.target_path, &hash, &patch_entry.target_hash).into());
  }
  let mut state = state.lock().unwrap();
  state.patch_files.0 += 1;
  drop(state);
  Ok(())
}


/*
 * Opens a file and calculates it's SHA256 hash
 */
fn get_hash(file_path: &str) -> String {
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
    patcher.set_location("C:/RenegadeX/".to_string());
    patcher.set_version_url("https://static.renegade-x.com/launcher_data/version/release.json".to_string());
    patcher.retrieve_mirrors().unwrap();
    patcher.remove_unversioned().unwrap();
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
