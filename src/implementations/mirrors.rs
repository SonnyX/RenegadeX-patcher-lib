use crate::structures::{Error, Mirror, Mirrors, NamedUrl};

use log::{trace, error};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU16, Ordering}};

use futures::future::join_all;

impl Mirrors {
  pub fn new(named_urls: Vec<NamedUrl>, version: String) -> Self {
    let mut mirrors = Vec::new();
    for mirror in &named_urls {
      if let Ok(url) = mirror.url.parse::<url::Url>() {
        if let Ok(ip) = url.socket_addrs(|| None) {
          mirrors.push(Mirror{
            address: Arc::new(format!("{}{}", &mirror.url, version)),
            ip: ip.into(),
            speed: 1.0,
            ping: 1000.0,
            error_count: Arc::new(AtomicU16::new(0)),
            enabled: Arc::new(AtomicBool::new(true)),
          });
        }
      }
    }
    Self {
      mirrors
    }
  }

    pub fn is_empty(&self) -> bool {
      self.mirrors.is_empty()
    }
  
    pub fn increment_error_count(&self, mirror: &Mirror) {
      let error_count = mirror.error_count.fetch_add(1, Ordering::Relaxed);
      if error_count == 3 {
        mirror.enabled.store(false, Ordering::Relaxed);
      }
    }
  
    pub fn remove(&self, mirror: Mirror) {
      mirror.enabled.store(false, Ordering::Relaxed);
    }
  
    pub fn disable(&self, entry: usize) {
      self.mirrors[entry].enabled.store(false, Ordering::Relaxed);
    }
  
    pub fn get_mirror(&self) -> Result<Mirror, Error> {
      for i in 0.. {
        for mirror in self.mirrors.iter() {
          if mirror.enabled.load(Ordering::Relaxed) && Arc::strong_count(&mirror.address) == i {
            trace!("i: {}, mirror: {}", i, &mirror.address);
            return Ok(mirror.clone());
          }
        }
      }
      return Err(Error::NoMirrors());
    }
  
    /**
    Checks the speed on the mirrors again
    */
    pub async fn test_mirrors(&mut self) -> Result<(), Error> {
      let mut handles = Vec::new();
      for i in 0..self.mirrors.len() {
        let mirror = self.mirrors[i].clone().test_mirror();
        handles.push(mirror);
      }
      let mirrors = join_all(handles).await;
      self.mirrors.clear();
      for result in mirrors {
        match result {
          Ok(mirror) => self.mirrors.push(mirror),
          Err(e) => error!("Testing mirror failed: {:?}", e)
        }
      }
      if self.mirrors.len() > 1 {
        self.mirrors.sort_by(|a,b| b.speed.partial_cmp(&a.speed).expect("mirrors.rs: Couldn't compare a.speed with b.speed."));
        let best_speed = self.mirrors[0].speed;
        for elem in self.mirrors.iter() {
          if elem.speed < best_speed / 4.0 {
            elem.enabled.store(false, Ordering::Relaxed);
          }
        }
      }
      Ok(())
    }
  }