use std::time::Duration;

use crate::{structures::{FilePart, Response, Mirror}, Error};

impl FilePart {
  async fn download(&self, mirror: Mirror) -> Result<Response, Error> {
    let mut downloader = download_async::Downloader::new();
    let uri = format!("{}/{}", mirror.address, self.file).parse::<download_async::http::Uri>()?;
    downloader.use_uri(uri);
    downloader.use_sockets(mirror.ip);
    let headers = downloader.headers().expect("Couldn't unwrap download_async headers option");
    headers.append("User-Agent", format!("RenX-Patcher ({})", env!("CARGO_PKG_VERSION")).parse().unwrap());
    headers.append("Range", format!("bytes={}-{}", &self.from, &self.to).parse().unwrap());
  
    let mut buffer = vec![];
    downloader.allow_http();
    let response = downloader.download(download_async::Body::empty(), &mut buffer);
  
    let result = tokio::time::timeout(Duration::from_secs(60), response).await??;
    Ok(Response::new(result, buffer))
  }
}