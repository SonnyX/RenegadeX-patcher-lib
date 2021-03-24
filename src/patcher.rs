//Standard library
use std::collections::BTreeMap;
use std::fs::{OpenOptions,DirBuilder};
use std::io::{Read, Write, Seek, SeekFrom};
use std::iter::FromIterator;
use std::ops::Deref;
use std::panic;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;

//Modules
use crate::downloader::BufWriter;
use crate::instructions::Instruction;
use crate::mirrors::{Mirrors, Mirror};
use crate::traits::{BorrowUnwrap, Error, ExpectUnwrap};
use crate::pausable::PausableTrait;
use crate::hashes::get_hash;
use crate::pausable::BackgroundService;
use crate::update::Update;
use crate::apply::apply_patch;
use crate::progress::Progress;
use crate::download_entry::DownloadEntry;
use crate::patch_entry::PatchEntry;
use crate::utilities::convert;


//External crates
use rayon::prelude::*;
use ini::Ini;
use log::*;
use download_async::Body;
use futures::task::AtomicWaker;
use futures::future::join_all;

//pub static LOL: Patcher = Patcher::new();


pub struct Patcher {
  pub logs: String,
  pub in_progress: Arc<AtomicBool>,
  pub join_handle: tokio::task::JoinHandle<()>,
}

impl Patcher {

  pub async fn get_remote_version() {

  }

  pub async fn start_validation() {

  }

  // information needed:
  //  renegadex_location: Option<String>,
  //  version_url: Option<String>,
  pub async fn start_patching(renegadex_location: String, version_url: String) -> Self {
    let join_handle = tokio::task::spawn(async {
      tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
      /*
      // Download release.json
      if self.version_and_mirror_info.is_empty() {
        self.version_and_mirror_info = download_single_file();
      }

      // Download instructions.json, however only if it hasn't been downloaded yet
      let instructions : Vec<Instruction> = download_instructions().await;

      // Sort instructions.json to be in groups.
      let instructionGroups : Vec<InstructionGroup> = instructions.sort();

      join_all(instructionGroups).pausable().await;
      // For each group:
      //   - check whether one of the files has a file matching with the new hash
      //   - otherwise with the old hash.
      //   - If no new hash exists:
      //     - Download delta or full file
      //     - Patch an old file
      //   - copy over the rest of the files

      // 
      */
    }.pausable());
  
    Self {
        logs: "".to_string(),
        in_progress: Arc::new(AtomicBool::new(true)),
        join_handle
      }
  }

  pub async fn cancel(self) -> Result<(), ()> {
    crate::pausable::FUTURE_CONTEXT.stop()?;
    let _ = self.join_handle.await;
    Ok(())
  }

  pub fn pause(&self) -> Result<(), ()> {
    crate::pausable::FUTURE_CONTEXT.pause()
  }

  pub fn resume(&self) -> Result<(), ()> {
    crate::pausable::FUTURE_CONTEXT.resume()
  }

  pub fn get_logs(&self) -> String {
    "".to_string()
  }
}



/*
pub async fn start() {

  async {
    // Download release.json
    if self.version_and_mirror_info.is_empty() {
      self.version_and_mirror_info = download_single_file();
    }

    // Download instructions.json, however only if it hasn't been downloaded yet
    let instructions : Vec<Instruction> = download_instructions().await;

    // Sort instructions.json to be in groups.
    let instructions : Vec<InstructionGroup> = instructions.sort();


    // For each group:
    //   - check whether one of the files has a file matching with the new hash
    //   - otherwise with the old hash.
    //   - If no new hash exists:
    //     - Download delta or full file
    //     - Patch an old file
    //   - copy over the rest of the files

    // 
  }.pausable().await

}
*/


#[derive(Debug)]
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

  pub fn get_launcher_info(&mut self) -> Option<crate::mirrors::LauncherInfo> {
    let ret = self.mirrors.launcher_info.clone();
    if ret.is_some() {
      self.mirrors.launcher_info.as_mut().unexpected("").prompted = true;
    }
    ret
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
  pub async fn retrieve_mirrors(&mut self) -> Result<(), Error> {
    if self.version_url.is_none() {
      Err("Version URL was not set before calling retrieve_mirrors".to_string().into())
    } else if self.mirrors.is_empty() {
      self.mirrors.get_mirrors(self.version_url.borrow()).await
    } else {
      Ok(())
    }
  }

  ///
  ///
  ///
  ///
  pub async fn rank_mirrors(&mut self) -> Result<(), Error> {
    if !self.mirrors.is_empty() {
      self.mirrors.test_mirrors().await?;
      info!("{:#?}", &self.mirrors.mirrors);
      Ok(())
    } else {
      Err("No mirrors available to test".to_string().into())
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
          let mut state = self.state.lock().unexpected("");
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
        let mut state = self.state.lock().unexpected("");
        state.update = Update::Full;
        drop(state);
        return Ok(Update::Full);
      }
    };

    let section = conf.section(Some("RenX_Game.Rx_Game".to_owned())).unexpected("");
    let game_version_number = section.get("GameVersionNumber").unexpected("");

    if self.mirrors.version_number.borrow() != game_version_number {
      let mut state = self.state.lock().unexpected("");
      state.update = Update::Delta;
      drop(state);
      return Ok(Update::Delta);
    }
    let mut state = self.state.lock().unexpected("");
    state.update = Update::UpToDate;
    drop(state);
    Ok(Update::UpToDate)
  }

  ///
  ///
  ///
  ///
  pub async fn download(&mut self) -> Result<(), Error> {
    // Reset progress
    let mut progress = self.state.lock().unexpected("");
    progress.update = Update::Unknown;
    progress.hashes_checked = (0,0);
    progress.download_size = (0,0);
    progress.patch_files = (0,0);
    progress.finished_hash = false;
    progress.finished_patching = false;
    // Drop the locked state object.
    drop(progress);

    
    self.download_hashmap = Mutex::new(BTreeMap::new());
    self.hash_queue = Mutex::new(Vec::new());
    self.patch_queue = Arc::new(Mutex::new(Vec::new()));

    if self.instructions.is_empty() {
      self.retrieve_instructions().await?;
    }
    self.process_instructions();
    info!("Retrieved instructions, checking hashes.");
    self.check_hashes()?;
    let child_process = self.check_patch_queue();
    self.download_files().await?;
    child_process.join().unexpected("");
    //need to wait somehow for patch_queue to finish.
    let mut state = self.state.lock().unexpected("");
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
  async fn retrieve_instructions(&mut self) -> Result<(), Error> {
    self.instructions = crate::instructions::retrieve_instructions(&self.mirrors).await?;
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
          if instruction.newest_hash.is_some() {
            let mut hash_queue = self.hash_queue.lock().unexpected("");
            hash_queue.push(instruction.clone());
            drop(hash_queue);
            let mut state = self.state.lock().unexpected("");
            state.hashes_checked.1 += 1;
            drop(state);
          } else {
            info!("Found entry {} that needs deleting.", instruction.path);
            //TODO: DeletionQueue, delete it straight away?
          }
        },
        Err(_e) => {
          if let Some(key) = &instruction.newest_hash {
            let delta_path = format!("{}patcher/{}", self.renegadex_location.borrow(), &key);
            let mut download_hashmap = self.download_hashmap.lock().unexpected("");
            if !download_hashmap.contains_key(key) {
              let download_entry = DownloadEntry {
                file_path: delta_path.clone(),
                file_size: instruction.full_vcdiff_size,
                file_hash: instruction.full_vcdiff_hash.clone().unexpected(""),
                patch_entries: Vec::new(),
              };
              download_hashmap.insert(key.clone(), download_entry);
              let mut state = self.state.lock().unexpected("");
              state.download_size.1 += instruction.full_vcdiff_size as u64;
              drop(state);
            }
            let patch_entry = PatchEntry {
              target_path: instruction.path.clone(),
              delta_path,
              has_source: false,
              target_hash: key.clone(),
            };
            download_hashmap.get_mut(key).unexpected("").patch_entries.push(patch_entry); //should we add it to a downloadQueue??
            drop(download_hashmap);
            let mut state = self.state.lock().unexpected("");
            state.patch_files.1 += 1;
            drop(state);
          }
        }
      };
    });
  }

  pub async fn remove_unversioned(&mut self) -> Result<(), Error> {
    if self.instructions.is_empty() {
      self.retrieve_instructions().await?;
    }
    crate::directory::remove_unversioned(&self.instructions, &self.renegadex_location.clone().unexpected("No RenX Location"))
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
  fn check_hashes(&mut self) -> Result<(), Error> {
    let hash_queue = self.hash_queue.lock().unexpected("");
    hash_queue.par_iter().for_each(|hash_entry| {
      let file_path_source = format!("{}.vcdiff_src", &hash_entry.path);
      let file_hash = match OpenOptions::new().read(true).open(&file_path_source) {
        Ok(_file) => {
          if hash_entry.previous_hash.is_some() && &get_hash(&file_path_source).unexpected("Failed to hash") == hash_entry.previous_hash.borrow() {
            match std::fs::remove_file(&hash_entry.path) {
              Ok(()) => {},
              Err(_e) => {
                error!("Couldn't remove file before renaming .vcdiff_src...");
              },
            }
            std::fs::rename(&file_path_source, &hash_entry.path).unexpected("");
          } else {
            match std::fs::remove_file(&file_path_source) {
              Ok(()) => {
                info!("Removed .vcdiff_src which did not match previous_hash...");
              },
              Err(_e) => {
                error!("Couldn't remove .vcdiff_src which did not match previous_hash...");
              }
            }
          }
          get_hash(&hash_entry.path).unexpected("Failed to hash")
        },
        Err(_e) => {
          get_hash(&hash_entry.path).unexpected("Failed to hash")
        },
      };
      if hash_entry.previous_hash.is_some() && hash_entry.newest_hash.is_some() && &file_hash == hash_entry.previous_hash.borrow() && &file_hash != hash_entry.newest_hash.borrow() && hash_entry.has_delta {
        //download patch file
        let key = format!("{}_from_{}", hash_entry.newest_hash.borrow(), hash_entry.previous_hash.borrow());
        let delta_path = format!("{}patcher/{}", self.renegadex_location.borrow(), &key);
        let mut download_hashmap = self.download_hashmap.lock().unexpected("");
        if !download_hashmap.contains_key(&key) {
          let download_entry = DownloadEntry {
            file_path: delta_path.clone(),
            file_size: hash_entry.delta_vcdiff_size,
            file_hash: match hash_entry.delta_vcdiff_hash.clone() {
              Some(hash) => hash,
              None => {
                error!("Delta hash is empty for download_entry: {:?}", hash_entry);
                panic!("Delta hash is empty for download_entry: {:?}", hash_entry)
              }
            },
            patch_entries: Vec::new(),
          };
          download_hashmap.insert(key.clone(), download_entry);
          let mut state = self.state.lock().unexpected("");
          state.download_size.1 += hash_entry.delta_vcdiff_size as u64;
          drop(state);
        }

        let patch_entry = PatchEntry {
          target_path: hash_entry.path.clone(),
          delta_path,
          has_source: true,
          target_hash: hash_entry.newest_hash.clone().unexpected(""),
        };
        download_hashmap.get_mut(&key).unexpected("").patch_entries.push(patch_entry);
        drop(download_hashmap);
        let mut state = self.state.lock().unexpected("");
        state.patch_files.1 += 1;
        state.hashes_checked.0 += 1;
        drop(state);
      } else if hash_entry.newest_hash.is_some() && &file_hash == hash_entry.newest_hash.borrow() {
        //this file is up to date
        let mut state = self.state.lock().unexpected("");
        state.hashes_checked.0 += 1;
        drop(state);
      } else {
        //this file does not match old hash, nor the new hash, thus it's corrupted
        //download full file
        trace!("No suitable patch file found for \"{}\", downloading full file!", &hash_entry.path);
        let key : &String = hash_entry.newest_hash.borrow();
        let delta_path = format!("{}patcher/{}", self.renegadex_location.borrow(), &key);
        let mut download_hashmap = self.download_hashmap.lock().unexpected("");
        if !download_hashmap.contains_key(key) {
         let download_entry = DownloadEntry {
            file_path: delta_path.clone(),
            file_size: hash_entry.full_vcdiff_size,
            file_hash: match hash_entry.full_vcdiff_hash.clone() {
              Some(hash) => hash,
              None => {
                error!("Delta hash is empty for download_entry: {:?}", hash_entry);
                panic!("Delta hash is empty for download_entry: {:?}", hash_entry)
              }
            },
            patch_entries: Vec::new(),
          };
          download_hashmap.insert(key.clone(), download_entry);
          let mut state = self.state.lock().unexpected("");
          state.download_size.1 += hash_entry.full_vcdiff_size as u64;
          drop(state);
        }

        let patch_entry = PatchEntry {
          target_path: hash_entry.path.clone(),
          delta_path,
          has_source: false,
          target_hash: hash_entry.newest_hash.clone().unexpected(""),
        };
        download_hashmap.get_mut(key).unexpected("").patch_entries.push(patch_entry);
        drop(download_hashmap);
        let mut state = self.state.lock().unexpected("");
        state.patch_files.1 += 1;
        state.hashes_checked.0 += 1;
        drop(state);
      }
    });
    self.state.lock().unexpected("").finished_hash = true;
    Ok(())
  }


/*
 * Iterates over the download_hashmap and calls download_and_patch for each DownloadEntry.
 */
  async fn download_files(&self) -> Result<(), Error> {
    let dir_path = format!("{}patcher/", self.renegadex_location.borrow());
    match DirBuilder::new().recursive(true).create(&dir_path) {
      Err(_) => {
        runas::Command::new("RenegadeX-folder-permissions.exe")
        .arg(format!("($acl = Get-ACL {directory}).AddAccessRule((New-Object System.Security.AccessControl.FileSystemAccessRule([System.Security.Principal.WindowsIdentity]::GetCurrent().Name,\"FullControl\",\"Allow\"))); $acl | Set-ACL {directory}", directory=self.renegadex_location.borrow()))
        .gui(true).spawn().unexpected("Could not set Access Rule for RenegadeX directory: Process did not launch.").wait().unexpected("Could not set Access Rule for RenegadeX directory: Process exited unexpectedly.");
        DirBuilder::new().recursive(true).create(&dir_path).unexpected("")
      },
      _ => {}
    }
    let download_hashmap = self.download_hashmap.lock().unexpected("");
    let mut sorted_downloads_by_size = Vec::from_iter(download_hashmap.deref());
    sorted_downloads_by_size.sort_unstable_by(|&(_, a), &(_,b)| b.file_size.cmp(&a.file_size));
    let mut handles = Vec::new();
    for (key, download_entry) in sorted_downloads_by_size.into_iter() {
      handles.push(self.download_and_patch(key, download_entry));
    }
    let results = futures::future::join_all(handles).await;
    for result in results {
      result?;
    }

    Ok(())
  }

  ///
  ///
  ///
  ///
  async fn download_and_patch(&self, key: &str, download_entry: &DownloadEntry) -> Result<(), Error> {
    for attempt in 0..5 {
      let mirror = self.mirrors.get_mirror();
      let download_url = match download_entry.patch_entries[0].has_source {
        true => format!("{}/delta/{}", &mirror.address, &key),
        false => format!("{}/full/{}", &mirror.address, &key)
      };
      match self.download_file(&mirror, &download_url, download_entry, attempt == 0).await {
        Ok(()) => {
          break
        },
        Err(e) => {
          warn!("Download {} failed with error message: {}", &download_url, e);
          self.mirrors.increment_error_count(&mirror);
          if attempt == 4 { return Err(format!("Couldn't download file: {}", &key).into()) }
          else {
            error!("Downloading file from {} failed due to error: {}", download_url, e);
            if e.remove_mirror {
              warn!("Removing mirror: {}", mirror.address);
              self.mirrors.remove(mirror);
            }
          }
        }
      };
    }
    info!("Adding {} to patch queue", &key);

    //apply delta
    let mut patch_queue = self.patch_queue.lock().unexpected("");
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
      let pool = rayon::ThreadPoolBuilder::new().num_threads(num_threads).build().unexpected("");
      pool.install(|| {
        rayon::scope(|s| {
          for _i in 0..num_threads {
            s.spawn(|_| {
              let state = unlocked_state.lock().unexpected("");
              let mut patch_files = state.patch_files;
              drop(state);
              while patch_files.0 != patch_files.1 {
                // Check for entry in patch_queue, get one, remove it, free the mutex, process entry.
                let patch_entries : Option<Vec<PatchEntry>>;
                {
                  let mut patch_queue = patch_queue_unlocked.lock().unexpected("");
                  patch_entries = patch_queue.pop();
                  drop(patch_queue);
                }
                if patch_entries.is_some() {
                  patch_entries.borrow().par_iter().for_each(|patch_entry| {
                    //println!("Patching with diff file: {}", &patch_entry.delta_path);
                    apply_patch(patch_entry, unlocked_state.clone()).unexpected("");
                    //println!("Patching success: {}", &patch_entry.delta_path);
                  });
                  std::fs::remove_file(patch_entries.borrow().first().unexpected("").delta_path.clone()).unexpected("");
                  let state = unlocked_state.lock().unexpected("");
                  patch_files = state.patch_files;
                  drop(state);
                } else {
                  std::thread::sleep(std::time::Duration::from_millis(20));
                  let state = unlocked_state.lock().unexpected("");
                  patch_files = state.patch_files;
                  drop(state);
                }
              }
            });
          }
        });
        {
          let mut state = unlocked_state.lock().unexpected("");
          state.finished_patching = true;
          drop(state);
        }
        //remove patcher folder and all remaining files in there:
        std::fs::remove_dir_all(format!("{}patcher/", renegadex_location.unexpected(""))).unexpected("");
      });
    })
  }

  ///
  /// Downloads the file in parts
  ///
  ///
  async fn download_file(&self, mirror: &Mirror, download_url: &str, download_entry: &DownloadEntry, first_attempt: bool) -> Result<(), Error> {
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
    if (f.metadata().unexpected("").len() as usize) < file_size {
      if f.metadata().unexpected("").len() == (download_entry.file_size as u64) {
        //If hash is correct, return.
        //Otherwise download again.
        let hash = get_hash(&download_entry.file_path)?;
        if hash == download_entry.file_hash {
          let mut state = self.state.lock().unexpected("");
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
    f.seek(SeekFrom::Start(download_entry.file_size as u64)).unexpected("");
    let mut buf = [0,0,0,0];
    f.read_exact(&mut buf).unexpected("");
    let resume_part : usize = u32::from_be_bytes(buf) as usize;
    if resume_part != 0 { 
      info!("Resuming download \"{}\" from part {} out of {}", &download_entry.file_path, resume_part, parts_amount);
      if first_attempt {
        let mut state = self.state.lock().unexpected("");
        state.download_size.0 += (part_size * resume_part) as u64;
        drop(state);
      }
    };

    self.get_file(&mirror, f, &download_url, resume_part, part_size, &download_entry).await?;
    //Let's make sure the downloaded file matches the Hash found in Instructions.json
    let hash = get_hash(&download_entry.file_path)?;
    if hash != download_entry.file_hash {
      let mut state = self.state.lock().unexpected("");
      state.download_size.0 -= download_entry.file_size as u64;
      drop(state);
      return Err(format!("File \"{}\"'s hash ({}) did not match with the one provided in Instructions.json ({})", &download_entry.file_path, &hash, &download_entry.file_hash).into());
    }
    Ok(())
  }

  async fn get_file(&self, mirror: &Mirror, f: std::fs::File, download_url: &str, resume_part: usize, part_size: usize, download_entry: &DownloadEntry ) -> Result<(), Error> {
    let unlocked_state = self.state.clone();
    get_download_file(unlocked_state, mirror, f, &download_url, resume_part, part_size, &download_entry).await
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
        let state = state.lock().unexpected("");
        finished_hash = state.finished_hash;
        finished_patching = state.finished_patching;
        let download_size : (u64, u64) = state.download_size;
        let patch_files : (u64, u64) = state.patch_files;
        let hashes_checked : (u64, u64) = state.hashes_checked;
        drop(state);
        if !finished_hash {
          if old_download_size != download_size {
            info!("Comparing files, total to be downloaded: {:.1} MB", (download_size.1 as f64)*0.000_001);
          }
          if old_hashes_checked != hashes_checked {
            info!("Checked {} out of {} hashes.", hashes_checked.0, hashes_checked.1);
          }
        } else {
          if old_download_size != download_size {
            info!("Downloaded {:.1}/{:.1} MB, speed: {}/s", (download_size.0 as f64)*0.000_001, (download_size.1 as f64)*0.000_001, convert((download_size.0 - old_download_size.0) as f64));
          }
          if patch_files != old_patch_files {
            info!("Patched {}/{} files", patch_files.0, patch_files.1);
          }
        }
        old_download_size = download_size;
        old_patch_files = patch_files;
        old_hashes_checked = hashes_checked;
      }
    });
  }


  /// Example usage:
  /// patcher.download_file("redists/UE3Redist.exe", writer);
  pub fn download_file_from_mirrors(&self, relative_path: &str, mut writer: impl Write + Send + 'static) -> Result<(), Error> {
    // Get a mirror:
    let mirror = self.mirrors.get_mirror();
    let ip = mirror.ip.clone();

    // Create the URL
    let mut url = format!("{}", mirror.address.to_owned());
    url.truncate(url.rfind('/').unexpected(&format!("mirrors.rs: Couldn't find a / in {}", &url)) + 1);
    let url = format!("{}{}", url, relative_path);
    trace!("{}", &url);
    let url = url.parse::<download_async::http::Uri>().unexpected("");
    
    // Set up the request
    let mut req = download_async::http::Request::builder();
    req = req.uri(url).header("User-Agent", "sonny-launcher/1.0");
    let req = req.body(Body::empty()).unexpected("");

    // Send the request
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unexpected("");
    let mut progress : Option<&mut crate::progress::DownloadProgress> = None;
    let result = download_async::download(req, &mut writer, false, &mut progress, Some(ip));
    rt.block_on(result)?;
    Ok(())
  }

  /// 
  /// 
  /// 
  /// 
  pub fn get_progress(&self) -> Arc<Mutex<Progress>> {
    self.state.clone()
  }
}

///
/// 
/// 
pub async fn get_download_file(unlocked_state: Arc<Mutex<Progress>>, mirror: &Mirror, f: std::fs::File, download_url: &str, resume_part: usize, part_size: usize, download_entry: &DownloadEntry ) -> Result<(), Error>  {
  let mut writer = BufWriter::new(f, move | file, total_written | {
    //When the buffer is being written to file, this closure gets executed
    let parts = *total_written / part_size as u64;
    file.seek(SeekFrom::End(-4)).unexpected("");
    file.write_all(&(parts as u32).to_be_bytes()).unexpected("");
    file.seek(SeekFrom::Start(*total_written)).unexpected("");
  });
  writer.seek(SeekFrom::Start((part_size * resume_part) as u64)).unexpected("");

  let url = download_url.parse::<download_async::http::Uri>().unexpected("");
  let trunc_size = download_entry.file_size as u64;

  let mut req = download_async::http::Request::builder();
  req = req.uri(url).header("User-Agent", "sonny-launcher/1.0");
  if resume_part != 0 {
    req = req.header("Range", format!("bytes={}-{}", (part_size * resume_part), download_entry.file_size));
  };
  let req = req.body(download_async::Body::empty()).unexpected("");
  let ip = mirror.ip.clone();

  let mut progress = crate::progress::DownloadProgress::new(unlocked_state);

  let result = download_async::download(req, &mut writer, false, &mut Some(&mut progress), Some(ip)).await;

  if result.is_ok() {
    let f = writer.into_inner()?;
    f.sync_all()?;
    f.set_len(trunc_size)?;
    Ok(())
  } else {
    Err(format!("Unexpected response: found status code!").into())
  }
}