extern crate reqwest;
extern crate json;
extern crate sha2;
extern crate hex;
extern crate ini;
extern crate rayon;
extern crate rand;

use ini::Ini;

use sha2::{Sha256, Digest};
use rayon::prelude::*;
use rand::Rng;

use std::io;
use std::io::{Read, Write, Seek, SeekFrom};
use std::fs::{File,OpenOptions,DirBuilder};
use std::time::{Duration, Instant};
use std::sync::Mutex;
use std::panic;

trait AsString {
  fn as_string(&self) -> Option<String>;
}

impl AsString for json::JsonValue {
    fn as_string(&self) -> Option<String> {
        match *self {
            json::JsonValue::Short(ref value)  => Some(value.to_string()),
            json::JsonValue::String(ref value) => Some(value.to_string()),
            _                                  => None
        }
    }
}

trait BorrowUnwrap<T> {
  fn borrow(&self) -> &T;
}

impl<T> BorrowUnwrap<T> for Option<T> {
  fn borrow(&self) -> &T {
    match self {
      Some(val) => val,
      None => panic!("called `Option::borrow()` on a `None` value"),
    }
  }
}

#[derive(Debug,Clone)]
pub struct Instruction {
  path: String,
  old_hash: Option<String>,
  new_hash: Option<String>,
  compressed_hash: Option<String>,
  delta_hash: Option<String>,
  old_last_write_time: String,
  new_last_write_time: String,
  full_replace_size: usize,
  delta_size: usize,
  has_delta: bool
}

#[derive(Debug)]
pub struct Mirror {
  address: String,
  speed: f64,
  ping: f64,
}
/*
pub smth DownloadQueue {
  Vec<hash_of_download mirrorlink isdelta>
}
*/
#[derive(Debug)]
pub struct PatchEntry {
  target_path: String,
  delta_path: String,
  has_source: bool,
  target_hash: String,
}

pub struct Downloader {
  pub RenegadeX_location: Option<String>, //Os dependant
  release_json: Option<json::JsonValue>, //release.json
  mirrors: Vec<Mirror>, //List of mirrors, sorted by their speed
  instructions: Vec<Instruction>, //instructions.json
  compressed_size: Option<u64>, //summed download size from instructions.json
  instructions_hash: Option<String>, //Hash of instructions.json
}

impl Downloader {
  pub fn new() -> Downloader {
    Downloader {
      RenegadeX_location: None,
      release_json: None,
      mirrors: Vec::new(),
      instructions: Vec::new(),
      compressed_size: None,
      instructions_hash: None,
    }
  }

  /**
  Checks if the version is the same in DefaultRenegadeX.ini as in release.json
  If not then return true, else return false.
  In the case that the game is not downloaded or DefaultRenegadeX.ini is missing, it will return yes.
  */
  pub fn update_available(&mut self) -> bool {
    if self.release_json.is_none() {
      self.get_release();
    }
    let release_data = self.release_json.clone().unwrap();
    let path = format!("{}UDKGame/Config/DefaultRenegadeX.ini", self.RenegadeX_location.borrow());
    let conf = match Ini::load_from_file(&path) {
      Ok(file) => file,
      Err(_e) => { return true }
    };

    let section = conf.section(Some("RenX_Game.Rx_Game".to_owned())).unwrap();
    let game_version_number = section.get("GameVersionNumber").unwrap();

    if &release_data["game"]["version_number"].as_u64().unwrap().to_string() != game_version_number {
      return true;
    }
    return false;
  }

  /**
  Downloads release.json from the renegade-x server and adds it to the struct
  */
  fn get_release(&mut self) {
    let mut release_json = match reqwest::get("https://static.renegade-x.com/launcher_data/version/release.json") {
      Ok(result) => result,
      Err(e) => panic!("Is your internet down? {}", e)
    };
    let release_json_response = match release_json.text() {
      Ok(result) => result,
      Err(e) => panic!("Corrupted response: {}", e)
    };
    let release_data = match json::parse(&release_json_response) {
      Ok(result) => result,
      Err(e) => panic!("Invalid JSON: {}", e)
    };
    self.release_json = Some(release_data.clone());

    //stop being a dick, and listen to sarah:
    let mut mirror_vec = Vec::with_capacity(release_data["game"]["mirrors"].len());
    release_data["game"]["mirrors"].members().for_each(|mirror| mirror_vec.push(mirror["url"].as_str().unwrap().to_string()) );
    let mirror_array : Vec<Mirror> = Vec::with_capacity(release_data["game"]["mirrors"].len());
    let data = Mutex::new(mirror_array);
    mirror_vec.par_iter().for_each(|mirror| {
      let mut url = mirror.clone();
      let http_client = reqwest::Client::builder().timeout(Duration::from_secs(1)).build().unwrap();
      url.push_str("10kb_file");
      let download_request = http_client.get(url.as_str());
      let start = Instant::now();
      let download_response = download_request.send();
      match download_response {
        Ok(result) => {
          let duration = start.elapsed();
          if result.headers()["content-length"] != "10000" { println!("{:?}", result); }
          let mirror_var = Mirror { 
            address: mirror.clone(),
            speed: (10000 as f64)/(duration.as_millis() as f64),
            ping: (duration.as_micros() as f64)/(1000 as f64),
          };
          data.lock().unwrap().push(mirror_var);
        },
        Err(_e) => {
          //this mirror will not be added
        }
      };
    });
    let mut mirror_array = data.into_inner().unwrap();
    mirror_array.sort_unstable_by(|a,b| b.speed.partial_cmp(&a.speed).unwrap());
    self.mirrors = mirror_array;
    self.instructions_hash = Some(String::from(release_data["game"]["instructions_hash"].as_str().unwrap()));
  }

  /**
  Downloads instructions.json from a mirror, checks its validity and if its valid it adds it to the struct
  */
  pub fn get_instructions(&mut self) {
    if self.release_json.is_none() {
      self.get_release();
    }
    let instructions_mutex : Mutex<String> = Mutex::new("".to_string());
    for retry in 0..3 {
      let result = panic::catch_unwind(|| {
        let instructions_url = format!("{}{}/instructions.json",
          &self.mirrors[retry].address, 
          &self.release_json.borrow()["game"]["patch_path"].as_str().unwrap());
        let mut instructions_response = match reqwest::get(&instructions_url) {
          Ok(result) => result,
          Err(e) => panic!("Is your internet down? {}", e)
        };
        let text = instructions_response.text().unwrap();
        // check instructions hash
        let mut sha256 = Sha256::new();
        sha256.input(&text);
        let hash = sha256.result();
        if &hash[..] != &hex::decode(self.release_json.borrow()["game"]["instructions_hash"].as_str().unwrap()).unwrap()[..] {
          panic!("Hashes did not match!");
        }
        *instructions_mutex.lock().unwrap() = text;
      });
      if result.is_ok() {
        for _i in 0..retry {
          println!("Removing mirror: {:#?}", &self.mirrors[0]);
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
    // hash matches with the one in release.json, copy to self.instructions
    let mut instruction_array : Vec<Instruction> = Vec::with_capacity(instructions_data.len());
    for i in 0..instructions_data.len() {
      instruction_array.push(Instruction {
        path: instructions_data[i]["Path"].as_string().unwrap(),
        old_hash: instructions_data[i]["OldHash"].as_string(),
        new_hash: instructions_data[i]["NewHash"].as_string(),
        compressed_hash: instructions_data[i]["CompressedHash"].as_string(),
        delta_hash: instructions_data[i]["DeltaHash"].as_string(),
        old_last_write_time: instructions_data[i]["OldLastWriteTime"].as_string().unwrap(),
        new_last_write_time: instructions_data[i]["NewLastWriteTime"].as_string().unwrap(),
        full_replace_size: instructions_data[i]["FullReplaceSize"].as_usize().unwrap(),
        delta_size: instructions_data[i]["DeltaSize"].as_usize().unwrap(),
        has_delta: instructions_data[i]["HasDelta"].as_bool().unwrap()
      });
    }
    self.instructions = instruction_array;
  }

  fn get_mirror(&self, entry: usize) -> String {
    format!("{}{}/", &self.mirrors[entry].address, self.release_json.borrow()["game"]["patch_path"].as_str().unwrap())
  }

  /**
  Iterates over the entries in instructions.json and does the following:
   * Checks if the file already exists
   * If the file exists compare the hash of the file with the OldHash
   * If the OldHash matches and there is a NewHash that is different, download delta.
   * Else download full file.
  */
  pub fn update(&mut self) {
    if self.instructions.len() == 0 {
      self.get_instructions();
    }
    let _release_json = self.release_json.clone().unwrap();
    DirBuilder::new().recursive(true).create(format!("{}/patcher/",&self.RenegadeX_location.borrow())).unwrap();
    let patch_queue : Mutex<Vec<PatchEntry>> = Mutex::new(Vec::with_capacity(self.instructions.len()));
    self.instructions.par_iter().for_each(|instruction| {
      let mut rng = rand::thread_rng();
      //Let's check NewHash if it is supposed to be Null, if it is then the file needs to be deleted.
      if instruction.new_hash.is_none() {
        let path = format!("{}{}", self.RenegadeX_location.borrow(), instruction.path.replace("\\","/"));
        match std::fs::remove_file(&path) {
          Ok(()) => (),
          Err(e) => println!("Couldn't remove file: {:?}", e) 
        };
      } else {
        //Compare the installed/existing files with the OldHash 
        let path = format!("{}{}", self.RenegadeX_location.borrow(), instruction.path.replace("\\","/"));
        match File::open(&path) {
          Ok(mut file) => {
            let mut sha256 = Sha256::new();
            io::copy(&mut file, &mut sha256).unwrap();
            let hash = sha256.result();
            //check if OldHash is some (not a new file), check if the file can be updated otherwise.
            if instruction.old_hash.is_some() && (&hash[..] == &hex::decode(instruction.old_hash.borrow()).unwrap()[..]) {
              //The installed file's hash is the same as the previous patch's hash
              if instruction.old_hash.borrow() != instruction.new_hash.borrow() {
                //a delta should be available, but let's make sure
                if instruction.has_delta {
                  for retry in 0..3 {
                    let mut mirror_entry : f32 = rng.gen();
                    mirror_entry *= 2.999;
                    mirror_entry = mirror_entry.floor();
                    match self.download_file(self.get_mirror(mirror_entry as usize), instruction.clone(), true) {
                      Ok(patch_entry) => {
                        patch_queue.lock().unwrap().push(patch_entry);
                        break
                      },
                      Err(e) => if retry == 2 { panic!("{}", e) }
                    };
                  }
                }
              }
            } else {
              //Old hash does not match the current file
              if &hash[..] != &hex::decode(instruction.new_hash.borrow()).unwrap()[..] {
                //Nor does it match the NewHash, thus a full file download is required.
                for retry in 0..3 {
                  let mut mirror_entry : f32 = rng.gen();
                  mirror_entry *= 2.999;
                  mirror_entry = mirror_entry.floor();
                  match self.download_file(self.get_mirror(mirror_entry as usize), instruction.clone(), false) {
                    Ok(patch_entry) => {
                      patch_queue.lock().unwrap().push(patch_entry);
                      break
                    },
                    Err(e) => if retry == 2 { panic!("{}", e) }
                  };
                }
              }
            }
          },
          Err(_e) => { 
            //Download full file
            for retry in 0..3 {
              let mut mirror_entry : f32 = rng.gen();
              mirror_entry *= 2.999;
              mirror_entry = mirror_entry.floor();
              match self.download_file(self.get_mirror(mirror_entry as usize), instruction.clone(), false) {
                Ok(patch_entry) => {
                  patch_queue.lock().unwrap().push(patch_entry);
                  break
                },
                Err(er) => if retry == 2 { panic!("{}", er) }
              };
            }
          }
        };
      }
    });
    let patch_queue = patch_queue.into_inner().unwrap();
    println!("{:#?}", patch_queue);
    //self.download_file(mirror, _instructions_json[0].clone(), true);
  }

  /**
  Downloads a file based on an entry from instructions.json, delta specifies if it has to be the delta or the full file.
  */
  fn download_file(&self, mirror: String, instruction: Instruction, delta: bool) -> Result<PatchEntry, &'static str> {
    let part_size :usize = 10u64.pow(6) as usize; //1.000.000
    //create a file in download location.
    let file_path = format!("{}/patcher/{}", &self.RenegadeX_location.borrow(), instruction.new_hash.borrow());
    let mut f = OpenOptions::new().read(true).write(true).create(true).open(&file_path).unwrap();
    //set the size of the file, add a 32bit integer to the end of the file as a means of tracking progress. We won't download parts async.
    let finished_file_size : usize = if delta { instruction.delta_size } else { instruction.full_replace_size };
    let parts_amount : usize = finished_file_size / part_size + if finished_file_size % part_size > 0 {1} else {0};
    let file_size : usize = finished_file_size + 4;
    if (f.metadata().unwrap().len() as usize) < file_size {
      if f.metadata().unwrap().len() == (finished_file_size as u64) {
        //If hash is correct, return.
        //Otherwise download again.
        let mut sha256 = Sha256::new();
        io::copy(&mut f, &mut sha256).unwrap();
        let hash = sha256.result();
        if &hash[..] == &hex::decode(if delta { instruction.delta_hash.borrow() } else { instruction.compressed_hash.borrow() }).unwrap()[..] {
          let patch_entry = PatchEntry {
            target_path: instruction.path,
            delta_path: file_path,
            has_source: delta,
            target_hash: instruction.new_hash.unwrap()
          };
          return Ok(patch_entry);
        }
      }
      println!("File size ({}) of patch file {} is smaller than it should be ({})",f.metadata().unwrap().len(), instruction.new_hash.borrow(), file_size);
      match f.set_len(file_size as u64) {
        Ok(()) => println!("Succesfully set file size"),
        Err(e) => {
          println!("Couldn't set file size! {}", e);
          return Err("Could not change file size of patch file, is it in use?");
        }
      }
    }
    let download_url = if delta {
                         format!("{}delta/{}_from_{}", &mirror, instruction.new_hash.borrow(), instruction.old_hash.borrow())
                       } else {
                         format!("{}full/{}", &mirror, instruction.new_hash.borrow())
                       };
    let http_client = reqwest::Client::new();
    f.seek(SeekFrom::Start((finished_file_size) as u64)).unwrap();
    let mut buf = [0,0,0,0];
    f.read_exact(&mut buf).unwrap();
    let resume_part : usize = u32::from_be_bytes(buf) as usize;
    if resume_part != 0 { println!("Resuming download from part: {}", resume_part) };
    //iterate over all parts, downloading them into memory, writing them into the file, adding one to the counter at the end of the file.
    let start = Instant::now();
    for part_int in resume_part..parts_amount {
      let bytes_start = part_int * part_size;
      let mut bytes_end = part_int * part_size + part_size -1;
      if bytes_end > finished_file_size {
        bytes_end = finished_file_size;
      }
      let download_request = http_client.get(&download_url).header(reqwest::header::RANGE,format!("bytes={}-{}", bytes_start, bytes_end));
      let download_response = download_request.send();
      f.seek(SeekFrom::Start(bytes_start as u64)).unwrap();
      let mut content : Vec<u8> = Vec::with_capacity(bytes_end - bytes_start + 1);
      download_response.unwrap().read_to_end(&mut content).unwrap();
      f.write_all(&content).unwrap();
      //completed downloading and writing this part, so update the progress-tracker at the end of the file
      f.seek(SeekFrom::Start((finished_file_size) as u64)).unwrap();
      f.write_all(&(part_int as u32).to_be_bytes()).unwrap();
    }
    let duration = start.elapsed();
    println!("Downloaded all parts!");
    println!("Average speed: {} kB/s!", (finished_file_size as f64)/(duration.as_millis() as f64));
    //Remove the counter at the end of the file to finish the vcdiff file
    f.set_len(finished_file_size as u64).unwrap();
    println!("Shrinked the file!");
    
    //Let's make sure the downloaded file matches the Hash found in Instructions.json
    f.seek(SeekFrom::Start(0)).unwrap();
    let mut sha256 = Sha256::new();
    io::copy(&mut f, &mut sha256).unwrap();
    let hash = sha256.result();
    if &hash[..] != &hex::decode(if delta { instruction.delta_hash.borrow() } else { instruction.compressed_hash.borrow() }).unwrap()[..] {
      println!("Hash is incorrect!");
      return Err("Downloaded file's hash did not match with the one provided in Instructions.json");
      //somehow restart the download :(
    }
    let patch_entry = PatchEntry {
      target_path: instruction.path,
      delta_path: file_path,
      has_source: delta,
      target_hash: instruction.new_hash.unwrap()
    };
    return Ok(patch_entry);
    //created a vcdiff library which is able to decompress this.
  }
}


#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn downloader() {
    let mut patcher : Downloader = Downloader::new();
    patcher.RenegadeX_location = Some("/home/sonny/RenegadeX/game_files/".to_string());
    let update : bool = patcher.update_available();
    patcher.get_instructions();
    //assert_eq!(update,true);
    patcher.update();
    assert!(true);
  }
}
