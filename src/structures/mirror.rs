use std::sync::{Arc, Mutex};
use download_async::SocketAddrs;

#[derive(Debug, Clone)]
pub struct Mirror {
  pub address: Arc<String>,
  pub speed: f64,
  pub ping: f64,
  pub error_count: Arc<Mutex<u16>>,
  pub enabled: Arc<Mutex<bool>>,
  pub ip: SocketAddrs,
}