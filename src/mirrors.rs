use std::time::{Duration, Instant};
use std::sync::Mutex;

use crate::traits::AsString;

use rayon::prelude::*;

#[derive(Debug)]
pub struct Mirror {
  pub address: String,
  pub speed: f64,
  pub ping: f64,
}

pub struct Mirrors {
  pub mirrors: Vec<Mirror>,
  pub instructions_hash: Option<String>,
  pub version_number: Option<String>,
}

impl Mirrors {
  pub fn new() -> Mirrors {
    Mirrors {
      mirrors: Vec::new(),
      instructions_hash: None,
      version_number: None,
    }
  }

  pub fn is_empty(&self) -> bool {
    if self.mirrors.len() == 0 {
      true
    } else {
      false
    }
  }

  pub fn remove(&mut self, entry: usize) {
    self.mirrors.remove(entry);
  }

  /**
  Downloads release.json from the renegade-x server and adds it to the struct
  */
  pub fn get_mirrors(&mut self, location: &String) {
    let mut release_json = match reqwest::get(location) {
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

    //stop being a dick, and listen to sarah:
    let mut mirror_vec = Vec::with_capacity(release_data["game"]["mirrors"].len());
    release_data["game"]["mirrors"].members().for_each(|mirror| mirror_vec.push(mirror["url"].as_string()) );
    let mirror_array : Vec<Mirror> = Vec::with_capacity(release_data["game"]["mirrors"].len());
    let data = Mutex::new(mirror_array);
    let patch_path = release_data["game"]["patch_path"].as_string();
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
            address: format!("{}{}", &mirror, &patch_path),
            speed: (10000 as f64)/(duration.as_millis() as f64),
            ping: (duration.as_micros() as f64)/(1000 as f64),
          };
          data.lock().unwrap().push(mirror_var);
        },
        Err(_e) => {
          //this mirror will not be added, error can thus be ignored.
        }
      };
    });
    let mut mirror_array = data.into_inner().unwrap();
    mirror_array.sort_unstable_by(|a,b| b.speed.partial_cmp(&a.speed).unwrap());
    self.mirrors = mirror_array;
    self.instructions_hash = Some(release_data["game"]["instructions_hash"].as_string());
    self.version_number = Some(release_data["game"]["version_number"].as_u64().unwrap().to_string());
  }
  
}
