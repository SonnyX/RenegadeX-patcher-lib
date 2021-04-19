use std::sync::{Arc,Mutex};
use std::time::{Duration, Instant};
use crate::structures::Mirror;
use crate::functions::download_file;

impl Mirror {
    pub(crate) async fn test_mirror(self) -> Mirror {
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