use std::time::{Duration, Instant};
use crate::structures::{Error, Mirror, Response};
use download_async;
use tracing::{instrument, Level};

impl Mirror {
  #[instrument(level = Level::INFO)]
  pub(crate) async fn test_mirror(self) -> Result<Mirror, Error> {
    let start = Instant::now();
    let download_response = self.download_file("10kb_file", Duration::from_secs(10)).await?;
    let duration = start.elapsed();
    let content_length = download_response.headers().get("content-length").ok_or_else(|| Error::None(format!("No header named: content_length")))?;

    if content_length != "10000" {
      return Err(Error::InvalidServer());
    }

    Ok(Mirror { 
      base: self.base,
      version: self.version,
      ip: self.ip,
      speed: 10_000.0/(duration.as_millis() as f64),
      ping: (duration.as_micros() as f64)/1000.0,
      error_count: self.error_count,
      enabled: self.enabled,
    })
  }

  #[instrument]
  pub(crate) async fn download_patchfile(&self, path: &str, timeout: Duration) -> Result<Response, Error> {
    self.download_file(&format!("{}/{}", self.version.to_string(), path), timeout).await
  }

  #[instrument]
  pub(crate) async fn download_file(&self, path: &str, timeout: Duration) -> Result<Response, Error> {
    let url = format!("{}{}", self.base.to_string(), path);

    let mut downloader = download_async::Downloader::new();
    //downloader.use_sockets(self.ip.clone());
    downloader.use_uri(url.parse::<download_async::http::Uri>()?);
    //let headers = downloader.headers().expect("Couldn't unwrap download_async headers option");
    //headers.append("User-Agent", format!("RenX-Patcher ({})", env!("CARGO_PKG_VERSION")).parse().unwrap());
    let mut buffer = vec![];
    //downloader.allow_http();
    let result = downloader.download(download_async::Body::empty(), &mut buffer).await?;

    //let result = tokio::time::timeout(timeout, response).await??;
    Ok(Response::new(result, buffer))
  }
}