use std::sync::{atomic::{AtomicU16, AtomicBool}, Arc};
use download_async::SocketAddrs;

#[derive(Debug, Clone)]
pub struct Mirror {
  pub base: Arc<String>,
  pub version: Arc<String>,
  pub speed: f64,
  pub ping: f64,
  pub error_count: Arc<AtomicU16>,
  pub enabled: Arc<AtomicBool>,
  pub ip: SocketAddrs,
}