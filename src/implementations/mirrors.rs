use std::time::Duration;

use crate::structures::{Error, Mirror, Mirrors, NamedUrl, SoftwareVersion};
use crate::functions::download_file;
use crate::traits::AsString;

use log::{trace, error};
use std::sync::{Arc, Mutex};

use futures::future::join_all;

impl Mirrors {
    pub fn new() -> Mirrors {
      Mirrors {
        mirrors: Vec::new(),
      }
    }
  
    pub fn is_empty(&self) -> bool {
      self.mirrors.is_empty()
    }
  
    pub fn increment_error_count(&self, entry: &Mirror) -> Result<(), Error> {
      for i in 0..self.mirrors.len() {
        if &self.mirrors[i].ip == &entry.ip {
          let error_count = self.mirrors[i].error_count.clone();
          let mut error_count = *error_count.lock().or_else(|_| Err(Error::MutexPoisoned(format!("mirrors.rs: Couldn't lock \"error_count\" field."))))?;
          error_count += 1;
          if error_count == 4 {
            self.disable(i);
          }
          break;
        }
      }
      Ok(())
    }
  
    pub fn remove(&self, entry: Mirror) -> Result<(), Error> {
      for i in 0..self.mirrors.len() {
        if self.mirrors[i].ip == entry.ip {
          self.disable(i);
          break;
        }
      }
      Ok(())
    }
  
    pub fn disable(&self, entry: usize)  -> Result<(), Error> {
      let enabled_mutex = self.mirrors[entry].enabled.clone();
      let mut enabled = *enabled_mutex.lock().or_else(|_| Err(Error::MutexPoisoned(format!("mirrors.rs: Couldn't lock the \"enabled\" field."))))?;
      enabled = false;
      Ok(())
    }
  
    pub async fn get_mirrors(&mut self, software: &SoftwareVersion) -> Result<(), Error> {
      for mirror in &software.mirrors {
        if let Ok(url) = mirror.url.parse::<url::Url>() {
          if let Ok(ip) = url.socket_addrs(|| None) {
            self.mirrors.push(Mirror{
              address: Arc::new(format!("{}{}", &mirror.url, software.version)),
              ip: ip.into(),
              speed: 1.0,
              ping: 1000.0,
              error_count: Arc::new(Mutex::new(0)),
              enabled: Arc::new(Mutex::new(false)),
            });
          }
        }
      }
      Ok(())
    }
  
    
    pub fn get_mirror(&self) -> Result<Mirror, Error> {
      for i in 0.. {
        for mirror in self.mirrors.iter() {
          if *mirror.enabled.lock().or_else(|_| Err(Error::None(format!("mirrors.rs: Couldn't get exclusive lock on mirror.enabled."))))? && Arc::strong_count(&mirror.address) == i {
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
          Err(e) => error!("Testing mirror failed: {}", e)
        }
      }
      if self.mirrors.len() > 1 {
        self.mirrors.sort_by(|a,b| b.speed.partial_cmp(&a.speed).expect("mirrors.rs: Couldn't compare a.speed with b.speed."));
        let best_speed = self.mirrors[0].speed;
        for elem in self.mirrors.iter() {
          if elem.speed < best_speed / 4.0 {
            *(elem.enabled.lock()?) = false;
          }
        }
      }
      Ok(())
    }
  }