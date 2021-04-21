use std::sync::{Arc,Mutex};
use std::time::{Duration, Instant};
use crate::structures::{Error, Mirror};
use crate::functions::download_file;

impl Mirror {
  pub(crate) async fn test_mirror(self) -> Result<Mirror, Error> {
    let start = Instant::now();
    let mut url = format!("{}", self.address.to_owned());
    url.truncate(url.rfind('/').ok_or_else(|| Error::None("Couldn't find a / in the url"))? + 1);
    url.push_str("10kb_file");
    let download_response = download_file(url, Duration::from_secs(2)).await?;
    let duration = start.elapsed();
    let content_length = download_response.headers().get("content-length").ok_or_else(|| Error::None("No header named: content_length"))?;

    if content_length != "10000" {
      return Err(Error::InvalidServer());
    }

    Ok(Mirror { 
      address: self.address,
      ip: self.ip,
      speed: 10_000.0/(duration.as_millis() as f64),
      ping: (duration.as_micros() as f64)/1000.0,
      error_count: self.error_count,
      enabled: self.enabled,
    })
  }
}