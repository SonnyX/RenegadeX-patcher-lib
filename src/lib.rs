extern crate reqwest;
extern crate json;
extern crate sha2;
extern crate hex;
extern crate ini;

use ini::Ini;

use sha2::{Sha256, Digest};

use std::process;
use std::io;
use std::io::{Read, Write, Seek, SeekFrom};
use std::fs::{File,OpenOptions,DirBuilder};

pub struct Downloader {
  RenegadeX_location: Option<String>, //Os dependant
  version: Option<String>, //RenegadeX version as mentioned in release.json
  release_json: Option<json::JsonValue>, //release.json
  instructions_json: Option<json::JsonValue>, //instructions.json
  compressed_size: Option<f64>, //summed download size from instructions.json
  instructions_hash: Option<String>, //Hash of instructions.json
}

impl Downloader {
  pub fn new() -> Downloader {
    let mut return_object = Downloader {
      RenegadeX_location: None,
      version: None,
      release_json: None,
      instructions_json: None,
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
    let path = format!("{}UDKGame/Config/DefaultRenegadeX.ini", self.RenegadeX_location.clone().unwrap());
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
      &self.release_json.clone().unwrap()["game"]["mirrors"][3]["url"].as_str().unwrap(), 
      &self.release_json.clone().unwrap()["game"]["patch_path"].as_str().unwrap());
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
    self.instructions_json = Some(instructions_data.clone());
  }

  /**
  Iterates over the entries in instructions.json and does the following:
   * Checks if the file already exists
   * If the file exists compare the hash of the file with the OldHash
   * If the OldHash matches and there is a NewHash that is different, download delta.
   * Else download full file.
  */
  pub fn update(&mut self) {
    if self.instructions_json.is_none() {
      self.get_instructions();
    }
    let _instructions_json = self.instructions_json.clone().unwrap();
    let _release_json = self.release_json.clone().unwrap();
    let mirror = format!("{}{}/", &_release_json["game"]["mirrors"][3]["url"].as_str().unwrap(), &_release_json["game"]["patch_path"].as_str().unwrap());
    DirBuilder::new().recursive(true).create(format!("{}/patcher/",&self.RenegadeX_location.clone().unwrap())).unwrap();

    for i in 0.._instructions_json.len() {
      //Let's check NewHash if it is supposed to be Null, if it is then the file needs to be deleted.
      if _instructions_json[i]["NewHash"].is_null() {
        let path = format!("{}{}", self.RenegadeX_location.clone().unwrap(), _instructions_json[i]["Path"].as_str().unwrap().replace("\\","/"));
        match std::fs::remove_file(&path) {
          Ok(()) => (),
          Err(e) => println!("Couldn't remove file: {:?}", e) 
        };
        continue;
      }
      //Compare the installed/existing files with the OldHash 
      let path = format!("{}{}", self.RenegadeX_location.clone().unwrap(), _instructions_json[i]["Path"].as_str().unwrap().replace("\\","/"));
      let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(e) => { 
          //Download full file
          self.download_file(mirror.clone(), _instructions_json[i].clone(), false);
          continue;
        }
      };
      let mut sha256 = Sha256::new();
      io::copy(&mut file, &mut sha256).unwrap();
      let hash = sha256.result();
      //check if OldHash is not Null (new file), check if the file can be updated otherwise.
      if _instructions_json[i]["OldHash"].is_null() == false && (&hash[..] == &hex::decode(_instructions_json[i]["OldHash"].as_str().unwrap()).unwrap()[..]) {
        //The installed file's hash is the same as the previous patch's hash
        if _instructions_json[i]["OldHash"].as_str().unwrap() != _instructions_json[i]["NewHash"].as_str().unwrap() {
          //a delta should be available, but let's make sure
          if _instructions_json[i]["HasDelta"].as_bool().unwrap() {
            self.download_file(mirror.clone(), _instructions_json[i].clone(), true);
          }
        }
      } else {
        //Old hash does not match the current file
        if &hash[..] != &hex::decode(_instructions_json[i]["NewHash"].as_str().unwrap()).unwrap()[..] {
          //Nor does it match the NewHash, thus a full file download is required.
          self.download_file(mirror.clone(), _instructions_json[i].clone(), false);
        }
      }
    }
    //self.download_file(mirror, _instructions_json[0].clone(), true);
  }

  /**
  Downloads a file based on an entry from instructions.json, delta specifies if it has to be the delta or the full file.
  */
  fn download_file(&self, mirror: String, file: json::JsonValue, delta: bool) {
    let part_size :usize = 10u64.pow(6) as usize; //1.000.000
    //create a file in download location.
    let file_path = format!("{}/patcher/{}", &self.RenegadeX_location.clone().unwrap(), &file["NewHash"].as_str().unwrap());
    let mut f = OpenOptions::new().read(true).write(true).create(true).open(&file_path).unwrap();
    //set the size of the file, add a 32bit integer to the end of the file as a means of tracking progress. We won't download parts async.
    let finished_file_size : usize = file[if delta {"DeltaSize"} else {"FullReplaceSize"}].as_usize().unwrap();
    let parts_amount : usize = finished_file_size / part_size + if finished_file_size % part_size > 0 {1} else {0};
    let file_size : usize = finished_file_size + 4;
    if (f.metadata().unwrap().len() as usize) < file_size {
      if f.metadata().unwrap().len() == (finished_file_size as u64) {
        //If hash is correct, return.
        //Otherwise download again.
        let mut sha256 = Sha256::new();
        io::copy(&mut f, &mut sha256).unwrap();
        let hash = sha256.result();
        if &hash[..] == &hex::decode(file[if delta {"DeltaHash"} else {"CompressedHash"}].as_str().unwrap()).unwrap()[..] {
          return;
        }
      }
      println!("File size ({}) of patch file {} is smaller than it should be ({})",f.metadata().unwrap().len(),file["NewHash"].as_str().unwrap(), file_size);
      match f.set_len(file_size as u64) {
        Ok(()) => println!("Succesfully set file size"),
        Err(e) => println!("Couldn't set file size! {}", e)
      }
    }
    let download_url = if delta {
                         format!("{}delta/{}_from_{}", &mirror, &file["NewHash"].as_str().unwrap(), &file["OldHash"].as_str().unwrap())
                       } else {
                         format!("{}full/{}", &mirror, &file["NewHash"].as_str().unwrap())
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
    //finally remove the counter at the end of the file
    f.set_len(finished_file_size as u64);
    println!("Shrinked the file!")

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
/*
  #[test]
  fn file_integrity() {
    let mut patcher : Downloader = Downloader::new();
    let update : bool = patcher.update_available();
    assert_eq!(update,true);
    patcher.RenegadeX_location = Some("/home/sonny/RenegadeX/game_files/".to_string());
    patcher.check_integrity();
    assert!(false);
  }
*/
/*
  #[test]
  fn sha() {
    let mut file = File::open("/home/sonny/git/CNC-Walls-correct.udk").unwrap();
    println!("opening file.");
    let mut sha256 = Sha256::new();
    io::copy(&mut file, &mut sha256).unwrap();
    let hash = sha256.result();
    println!("hash is: {:x}", hash);
  }
*/
}
