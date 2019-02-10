extern crate reqwest;
extern crate json;
extern crate sha2;
extern crate hex;
extern crate ini;
extern crate rayon;

use ini::Ini;

use sha2::{Sha256, Digest};
use rayon::prelude::*;

use std::process;
use std::io;
use std::io::{Read, Write, Seek, SeekFrom};
use std::fs::{File,OpenOptions,DirBuilder};

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

/*
pub struct Progress {
  entry:
  percentage
}

pub struct ProgressArray {
  
}
*/

pub struct Downloader {
  RenegadeX_location: Option<String>, //Os dependant
  version: Option<String>, //RenegadeX version as mentioned in release.json
  release_json: Option<json::JsonValue>, //release.json
  instructions: Vec<Instruction>, //instructions.json
  compressed_size: Option<f64>, //summed download size from instructions.json
  instructions_hash: Option<String>, //Hash of instructions.json
}

impl Downloader {
  pub fn new() -> Downloader {
    let mut return_object = Downloader {
      RenegadeX_location: None,
      version: None,
      release_json: None,
      instructions: Vec::new(),
      compressed_size: None,
      instructions_hash: None,
    };
    return return_object

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
    let mut file = match File::open(&path) {
      Ok(file) => file,
      Err(e) => { return true }
    };
    let conf = Ini::load_from_file(&path).unwrap();

    let section = conf.section(Some("RenX_Game.Rx_Game".to_owned())).unwrap();
    let game_version_number = section.get("GameVersionNumber").unwrap();
    let game_version = section.get("GameVersion").unwrap();

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
    self.instructions_hash = Some(String::from(release_data["game"]["instructions_hash"].as_str().unwrap()));
  }

  /**
  Downloads instructions.json from a mirror, checks its validity and if its valid it adds it to the struct
  */
  fn get_instructions(&mut self) {
    if self.release_json.is_none() {
      self.get_release();
    }
    let mut instructions_url = format!("{}{}/instructions.json",
      &self.release_json.borrow()["game"]["mirrors"][3]["url"].as_str().unwrap(), 
      &self.release_json.borrow()["game"]["patch_path"].as_str().unwrap());
    let mut instructions_response = match reqwest::get(&instructions_url) {
      Ok(result) => result,
      Err(e) => panic!("Is your internet down? {}", e)
    };
    let instructions_text = match instructions_response.text() {
      Ok(result) => result,
      Err(e) => panic!("Corrupted response: {}", e)
    };
    let instructions_data = match json::parse(&instructions_text) {
      Ok(result) => result,
      Err(e) => panic!("Invalid JSON: {}", e)
    };

    let mut InstructionArray : Vec<Instruction> = Vec::with_capacity(instructions_data.len());
    for i in 0..instructions_data.len() {
      InstructionArray.push(Instruction {
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

    self.instructions = InstructionArray;
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
    let mirror = format!("{}{}/", &_release_json["game"]["mirrors"][3]["url"].as_str().unwrap(), &_release_json["game"]["patch_path"].as_str().unwrap());
    DirBuilder::new().recursive(true).create(format!("{}/patcher/",&self.RenegadeX_location.borrow())).unwrap();
    self.instructions.par_iter().for_each(|instruction| {
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
                    match self.download_file(mirror.clone(), instruction.clone(), true) {
                      Ok(()) => break,
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
                  match self.download_file(mirror.clone(), instruction.clone(), false) {
                    Ok(()) => break,
                    Err(e) => if retry == 2 { panic!("{}", e) }
                  };
                }
              }
            }
          },
          Err(e) => { 
            //Download full file
            for retry in 0..3 {
              match self.download_file(mirror.clone(), instruction.clone(), false) {
                Ok(()) => break,
                Err(e) => if retry == 2 { panic!("{}", e) }
              };
            }
          }
        };
      }
    });
    //self.download_file(mirror, _instructions_json[0].clone(), true);
  }

  /**
  Downloads a file based on an entry from instructions.json, delta specifies if it has to be the delta or the full file.
  */
  fn download_file(&self, mirror: String, instruction: Instruction, delta: bool) -> Result<(), &'static str> {
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
          return Ok(());
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
    f.seek(SeekFrom::Start((finished_file_size) as u64));
    let mut buf = [0,0,0,0];
    f.read_exact(&mut buf);
    let resume_part : usize = u32::from_be_bytes(buf) as usize;
    if resume_part != 0 { println!("Resuming download from part: {}", resume_part) };
    //iterate over all parts, downloading them into memory, writing them into the file, adding one to the counter at the end of the file.
    for part_int in resume_part..parts_amount {
      let bytes_start = part_int * part_size;
      let mut bytes_end = part_int * part_size + part_size -1;
      if bytes_end > finished_file_size {
        bytes_end = finished_file_size;
      }
      let mut download_request = http_client.get(&download_url).header(reqwest::header::RANGE,format!("bytes={}-{}", bytes_start, bytes_end));
      let mut download_response = download_request.send();
      f.seek(SeekFrom::Start(bytes_start as u64));
      let mut content : Vec<u8> = Vec::with_capacity(bytes_end - bytes_start + 1);
      download_response.unwrap().read_to_end(&mut content);
      f.write_all(&content);
      //completed downloading and writing this part, so update the progress-tracker at the end of the file
      f.seek(SeekFrom::Start((finished_file_size) as u64));
      f.write_all(&(part_int as u32).to_be_bytes());
    }
    println!("Downloaded all parts!");
    //Remove the counter at the end of the file to finish the vcdiff file
    f.set_len(finished_file_size as u64);
    println!("Shrinked the file!");
    
    //Let's make sure the downloaded file matches the Hash found in Instructions.json
    f.seek(SeekFrom::Start(0));
    let mut sha256 = Sha256::new();
    io::copy(&mut f, &mut sha256).unwrap();
    let hash = sha256.result();
    if &hash[..] != &hex::decode(if delta { instruction.delta_hash.borrow() } else { instruction.compressed_hash.borrow() }).unwrap()[..] {
      println!("Hash is incorrect!");
      return Err("Downloaded file's hash did not match with the one provided in Instructions.json");
      //somehow restart the download :(
    }
    return Ok(());
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
    assert_eq!(update,true);
    patcher.update();
    assert!(false);
  }
}
