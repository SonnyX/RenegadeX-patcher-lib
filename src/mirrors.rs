use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use download_async::SocketAddrs;

use crate::downloader::download_file;
use crate::traits::{AsString,Error,ExpectUnwrap};
use futures::future::join_all;
use log::{error,trace};


#[derive(Debug, Clone)]
pub struct Mirror {
  pub address: Arc<String>,
  pub speed: f64,
  pub ping: f64,
  pub error_count: Arc<Mutex<u16>>,
  pub enabled: Arc<Mutex<bool>>,
  pub ip: SocketAddrs,
}

impl Mirror {
  async fn test_mirror(self) -> Mirror {
    let start = Instant::now();
    let mut url = format!("{}", self.address.to_owned());
    url.truncate(url.rfind('/').unexpected(&format!("mirrors.rs: Couldn't find a / in {}", &url)) + 1);
    url.push_str("10kb_file");
    let download_response = download_file(url, Duration::from_secs(2)).await;
    match download_response {
      Ok(result) => {
        let duration = start.elapsed();
        let content_length = result.headers().get("content-length");
        if content_length.is_none() || content_length.unexpected("mirrors.rs: Couldn't unwrap content_length") != "10000" {
          Mirror { 
            address: self.address,
            ip: self.ip,
            speed: 0.0,
            ping: 1000.0,
            error_count: Arc::new(Mutex::new(0)),
            enabled: Arc::new(Mutex::new(false)),
          }
        } else {
          Mirror { 
            address: self.address,
            ip: self.ip,
            speed: 10_000.0/(duration.as_millis() as f64),
            ping: (duration.as_micros() as f64)/1000.0,
            error_count: Arc::new(Mutex::new(0)),
            enabled: Arc::new(Mutex::new(true)),
          }
        }
      },
      Err(_e) => {
        Mirror { 
          address: self.address,
          ip: self.ip,
          speed: 0.0,
          ping: 1000.0,
          error_count: Arc::new(Mutex::new(0)),
          enabled: Arc::new(Mutex::new(false)),
        }
      }
    }
  }
}

#[derive(Debug, Clone)]
pub struct LauncherInfo {
  pub version_name: String,
  pub version_number: usize,
  pub patch_url: String,
  pub patch_hash: String,
  pub prompted: bool,
}

#[derive(Debug)]
pub struct Mirrors {
  pub mirrors: Vec<Mirror>,
  pub instructions_hash: Option<String>,
  pub version_number: Option<String>,
  pub launcher_info: Option<LauncherInfo>,
}

impl Mirrors {
  pub fn new() -> Mirrors {
    Mirrors {
      mirrors: Vec::new(),
      instructions_hash: None,
      version_number: None,
      launcher_info: None,
    }
  }

  pub fn is_empty(&self) -> bool {
    self.mirrors.is_empty()
  }

  pub fn increment_error_count(&self, entry: &Mirror) {
    for i in 0..self.mirrors.len() {
      if &self.mirrors[i].ip == &entry.ip {
        let error_count = self.mirrors[i].error_count.clone();
        *error_count.lock().unexpected("mirrors.rs: Couldn't lock error_count field.") += 1;
        if *error_count.lock().unexpected("mirrors.rs: Couldn't lock error_count field.") == 4 {
          self.disable(i);
        }
      }
    }
  }

  pub fn remove(&self, entry: Mirror) {
    for i in 0..self.mirrors.len() {
      if self.mirrors[i].ip == entry.ip {
        self.disable(i);
      }
    }
  }

  pub fn disable(&self, entry: usize) {
    let mirrors = self.mirrors[entry].enabled.clone();
    *mirrors.lock().unexpected("mirrors.rs: Couldn't lock enabled field.") = false;
  }

  /**
  Downloads release.json from the renegade-x server and adds it to the struct
  */
  pub async fn get_mirrors(&mut self, location: &str) -> Result<(), Error> {
    let mut release_json = match download_file(location.to_string(), Duration::from_secs(10)).await {
      Ok(result) => result,
      Err(e) => return Err(format!("Is your internet down? {}", e).into())
    };
    let release_json_response = match release_json.text() {
      Ok(result) => result,
      Err(e) => return Err(format!("mirrors.rs: Corrupted response: {}", e).into())
    };
    let release_data = match json::parse(&release_json_response) {
      Ok(result) => result,
      Err(e) => return Err(format!("mirrors.rs: Invalid JSON: {}", e).into())
    };
    self.launcher_info = Some(LauncherInfo {
      version_name: release_data["launcher"]["version_name"].as_string(),
      version_number: release_data["launcher"]["version_number"].as_usize().unexpected(&format!("mirrors.rs: Could not cast JSON version_number as a usize, input was {}", release_data["game"]["version_number"])),
      patch_url: release_data["launcher"]["patch_url"].as_string(),
      patch_hash: release_data["launcher"]["patch_hash"].as_string(),
      prompted: false,
    });
    let mut mirror_vec = Vec::with_capacity(release_data["game"]["mirrors"].len());
    release_data["game"]["mirrors"].members().for_each(|mirror| mirror_vec.push(mirror["url"].as_string()) );
    for mirror in mirror_vec {
      if let Ok(url) = mirror.parse::<url::Url>() {
        if let Ok(ip) = url.socket_addrs(|| None) {
          self.mirrors.push(Mirror{
            address: Arc::new(format!("{}{}", &mirror, release_data["game"]["patch_path"].as_string())),
            ip: ip.into(),
            speed: 1.0,
            ping: 1000.0,
            error_count: Arc::new(Mutex::new(0)),
            enabled: Arc::new(Mutex::new(false)),
          });
        }
      }
    }

    self.instructions_hash = Some(release_data["game"]["instructions_hash"].as_string());
    self.version_number = Some(release_data["game"]["version_number"].as_u64().unexpected(&format!("mirrors.rs: Could not cast JSON version_number as a u64, input was {}", release_data["game"]["version_number"])).to_string());
    Ok(())
  }

  
  pub fn get_mirror(&self) -> Mirror {
    for i in 0.. {
      for mirror in self.mirrors.iter() {
        if *mirror.enabled.lock().unexpected("mirrors.rs: Couldn't get exclusive lock on mirror.enabled.") && Arc::strong_count(&mirror.address) == i {
          trace!("i: {}, mirror: {}", i, &mirror.address);
          return mirror.clone();
        }
      }
    }
    error!("No mirrors were found!");
    panic!("No mirrors found?");
  }

  /**
  Checks the speed on the mirrors again
  */
  pub async fn test_mirrors(&mut self) -> Result<(), Error> {
    let mut handles = Vec::new();
    for i in 0..self.mirrors.len() {
      let mirror = self.mirrors[i].clone();
      handles.push(mirror.test_mirror());
    }
    let mirrors = join_all(handles).await;
    for mirror in mirrors {
      for i in 0..self.mirrors.len() {
        if self.mirrors[i].address == mirror.address {
          self.mirrors[i] = mirror;
          break;
        }
      }
    }
    if self.mirrors.len() > 1 {
      self.mirrors.sort_by(|a,b| b.speed.partial_cmp(&a.speed).unexpected(&format!("mirrors.rs: Couldn't compare a.speed with b.speed.")));
      let best_speed = self.mirrors[0].speed;
      for mut elem in self.mirrors.iter_mut() {
        if elem.speed < best_speed / 4.0 {
          elem.enabled = Arc::new(Mutex::new(false));
        }
      }
    }
    Ok(())
  }
}
