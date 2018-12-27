extern crate reqwest;
extern crate json;

use std::process;
use std::io;
use std::fs::File;

pub struct Downloader {
  version: Option<String>,
  mirrors: Option<json::JsonValue>,
  compressed_size: Option<f64>,
  instructions_hash: Option<String>
  //
}

impl Downloader {
  pub fn new() -> Downloader {
    Downloader {
      version: None,
      mirrors: None,
      compressed_size: None,
      instructions_hash: None
    }
    //try to locate patcher log
    //if not found initialize struct with empty values?
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
    self.mirrors = Some(release_data["game"]["mirrors"].clone());
    let instructions_hash = release_data["game"]["instructions_hash"].as_str().unwrap();
    let saved_instructions_hash : &str = &"hai";
    if &instructions_hash != &saved_instructions_hash {
      println!("New instructions found: {} vs {}", &instructions_hash, &saved_instructions_hash);
      return true;
    }
    println!("No new instructions found: {} vs {}", &instructions_hash, &saved_instructions_hash);
    return false;
  }

  pub fn download_full(&mut self) {
    //mirrors are aquired, ping them to see which is fast, which isn't?
  }

  pub fn download_file(&mut self, index: u64) {
    
  }

  pub fn download_patch(&mut self) {
    
  }
}

pub struct Launcher {
  //for example: ~/RenegadeX/
  RenegadeX_location: Option<String>,
  //for example: ~/RenegadeX/wine/
  wine_location: Option<String>,
  //for example: ~/RenegadeX/instance/
  wine_prefix: Option<String>,
  //for example: tkg-protonified-3.21
  wine_version: Option<String>,
  //For example: DRI_PRIME=1
  env_arguments: Option<String>,
  servers: Option<json::JsonValue>,
  ping: Option<json::JsonValue>
}

impl Launcher {
  fn download_wine(&mut self, version: String) {
    //grab wine version from play-on-linux, wine itself, or from lutris.
    //...
    //Install required packages, we probably are able to ditch .net since we do no longer need the launcher.
    //At some point we may be able to use VK9 to improve performance.
    //
  }

  //Checks if the (paranoid) kernel blocks ICMP by programs such as wine, otherwise prompt the user to enter password to execute the followiwng commands
  fn ping_test(&mut self) {
    let successful = false;
    if successful {
      /*
Need to use polkit somehow to show the user a dialog questioning to allow executing setcap in order to allow the launcher (and wine?) to ping.
      https://wiki.archlinux.org/index.php/Polkit

      sudo setcap cap_net_raw+epi /usr/bin/wine-preloader
      sudo setcap cap_net_raw+epi /usr/bin/wine
      sudo setcap cap_net_raw+epi /usr/bin/wine64-preloader
      sudo setcap cap_net_raw+epi /usr/bin/wine64
      */
    }
  }

  //Checks if wine prefix exists, if not create it, install necessary libraries.
  fn instantiate_wine_prefix(&mut self) {
    
  }

  pub fn refresh_server_list(&mut self) {
    
  }

  pub fn launch_game(&mut self, server_index: Option<u16>) {
    if server_index == None {
      let mut wine_location = self.RenegadeX_location.clone().unwrap();
      wine_location.push_str("libs/wine/bin/wine");
      let mut game_location = self.RenegadeX_location.clone().unwrap();
      game_location.push_str("game_files/Binaries/Win64/UDK.exe");
      let mut wine = process::Command::new(wine_location)
      .arg(game_location)
      .stdout(process::Stdio::piped())
      .stderr(process::Stdio::inherit())
      .spawn().expect("failed to execute child");
    } else {
      let mut wine_location = self.RenegadeX_location.clone().unwrap();
      wine_location.push_str("libs/wine/bin/wine");
      let mut game_location = self.RenegadeX_location.clone().unwrap();
      game_location.push_str("game_files/Binaries/Win64/UDK.exe");

      let mut wine = process::Command::new(wine_location)
      .arg(game_location)
      .arg("some server")
      .stdout(process::Stdio::piped())
      .stderr(process::Stdio::inherit())
      .spawn().expect("failed to execute child");
    }
  }

}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn Downloader() {
    let mut patcher : Downloader = Downloader::new();
    let update : bool = patcher.update_available();
    println!("{}", patcher.mirrors.unwrap().pretty(2 as u16));
    assert_eq!(update,true);
    assert_eq!(update,false);
    assert_eq!(2 + 2, 4);
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
