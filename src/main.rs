extern crate reqwest;
extern crate json;
extern crate clap;
extern crate bit_vec;

use clap::App;
use clap::Arg;

use std::process;
use std::io;
use bit_vec::BitVec;
use std::fs::{OpenOptions,DirBuilder};

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

  pub fn update_available(&mut self) -> bool {
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
    let instructions_hash = release_data["game"]["instructions_hash"].as_str().unwrap();
    let saved_instructions_hash : &str = &"hai";
    if &instructions_hash != &saved_instructions_hash {
      println!("New instructions found: {} vs {}", &instructions_hash, &saved_instructions_hash);
      return true;
    }
    println!("No new instructions found: {} vs {}", &instructions_hash, &saved_instructions_hash);
    return false;
  }

  pub fn update(&mut self) {
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
    let mirror = format!("{}{}/",&self.release_json.clone().unwrap()["game"]["mirrors"][3]["url"].as_str().unwrap(), &self.release_json.clone().unwrap()["game"]["patch_path"].as_str().unwrap());
    DirBuilder::new().recursive(true).create(format!("{}/patcher/",&self.RenegadeX_location.clone().unwrap())).unwrap();
    self.download_file(mirror, instructions_data[0].clone());
  }

  pub fn progress(&self) {
    
  }

  pub fn download_full(&mut self) {
    //mirrors are aquired, ping them to see which is fast and which isn't?
  }

  fn download_file(&self, mirror: String, file: json::JsonValue) {
    let part_size = 10u64.pow(4); //10.000 as u64
    //create a .part file in download location.
    let file_path = format!("{}/patcher/{}", &self.RenegadeX_location.clone().unwrap(), &file["NewHash"].as_str().unwrap());
    let mut f = OpenOptions::new().read(true).write(true).create(true).open(&file_path).unwrap();
    let file_size = file["FullReplaceSize"].as_u64().unwrap();
    let parts_amount = (file_size / part_size + if file_size % part_size > 0 {1} else {0}) as usize;
    let mut parts = BitVec::from_elem(parts_amount, false);
    
    if f.metadata().unwrap().len() < file_size + parts_amount {
      println!("File size ({}) of patch file {} is smaller than it should be ({})",f.metadata().unwrap().len(),file["NewHash"].as_str().unwrap(), file_size + parts_amount);
      match f.set_len(file_size + parts_amount) {
        Ok(()) => println!("Succesfully set file size"),
        Err(e) => println!("Couldn't set file size! {}", e)
      }
    }
    let download_url = format!("{}full/{}", &mirror, &file["NewHash"].as_str().unwrap());
    let http_client = reqwest::Client::new();
    let mut download_request = http_client.get(&download_url).header(reqwest::header::RANGE,"bytes=0-100");
    println!("{:?}", download_request);
    let mut download_response = download_request.send();
    println!("{:?}", download_response);
    println!("{:?}", download_response.unwrap().text());
  }

  pub fn download_patch(&mut self) {
    
  }
}

fn main() {
  let matches = App::new("RenegadeX downloader/patcher")
    .author("Author: Randy von der Weide")
    .arg(Arg::with_name("check")
      .short("c")
      .long("check")
      .help("Checks if game is installed or if an update is available")
    )
    .arg(Arg::with_name("update")
      .short("u")
      .long("update")
      .help("Downloads and installs update if available")
    )
    .arg(Arg::with_name("RENX_PATH")
      .help("The location where RenegadeX is installed or should be installed")
      .required(true)
      .index(1))
    .get_matches();
  let mut patcher : Downloader = Downloader::new();
  
}


#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn downloader() {
    let mut patcher : Downloader = Downloader::new();
    let update : bool = patcher.update_available();
    assert_eq!(update,true);
    patcher.RenegadeX_location = Some("/home/sonny/git/Renegade-X-patcher-lib/RenegadeX".to_string());
    patcher.update();
    assert!(false);

    //println!("{}", patcher.mirrors.unwrap().pretty(2 as u16));
 

  }
}


/*
pub fn update_game() -> Result<(), reqwest::Error> {
  //TODO: check if instuctions_hash has changed since last time the game was started and if the previous update was succesfully completed.
  let mirrors = &release_data["game"]["mirrors"];
  let mirror_url = format!("{}{}/", &mirrors[0]["url"], &release_data["game"]["patch_path"]);
  let instructions_url = format!("{}instructions.json", &mirror_url);
  println!("Downloading instructions.json:");
  let mirror_response = reqwest::get(&instructions_url)?.text()?;
  println!("Downloading complete! Rustifying!");
  let mirror_data = json::parse(&mirror_response).unwrap();
  println!("Rustifying complete! Showing first entry:");
  println!("{}", &mirror_data[0]);
  //probably the part where tokio should kick in!

  let first_file_download_url = format!("{}full/{}",&mirror_url,&mirror_data[0]["NewHash"]);
  let mut first_file_download_response = reqwest::get(&first_file_download_url)?;
  println!("Downloaded first file into memory!");
  let mut file_delta: Vec<u8> = vec![];
  let file_delta_size = match first_file_download_response.copy_to(&mut file_delta) {
    Ok(result) => result,
    Err(e) => panic!("Copy failed: {}", e)
  };
  if file_delta_size != mirror_data[0]["FullReplaceSize"].as_u64().unwrap() {
    panic!("delta file does not match the correct size.");
  }
  
  let mut slice: &[u8] = &file_delta;
  let mut dest = {
    let fname = "/home/sonny/eclipse-workspace/renegade_x_launcher/delta";
        match File::create(&fname) {
          Ok(file) => file,
          Err(e) => panic!("Error!")
        }
  };
  match io::copy(&mut slice, &mut dest) {
    Ok(o) => o,
    Err(e) => panic!("Error!")
  };
  //Using command-line interface to decode files, nasty solution as it is not cross-platform compatible out of the box, might create a vcdiff library which is able to decompress this.
  let mut xdelta = process::Command::new(Some("xdelta3").unwrap())
      .arg("-d")
      .arg("/home/sonny/eclipse-workspace/renegade_x_launcher/delta")
      .arg("/home/sonny/eclipse-workspace/renegade_x_launcher/output")
      .stdout(process::Stdio::piped())
      .stderr(process::Stdio::inherit())
      .spawn().expect("failed to execute child");
  if !xdelta.wait().expect("failed to wait on child").success() {
    println!("Failed to decompile");
  }
  Ok(())
}
*/
