use std::{time::Duration, io::SeekFrom};

use download_async::http::StatusCode;
use tracing::warn;
use std::{fs::OpenOptions, io::{Write, Seek}};

use crate::{structures::{FilePart, Mirrors}, Error, Progress};

impl FilePart {
  pub(crate) async fn download(self, mirrors: Mirrors, mirror_path: String, progress: Progress) -> Result<(Self, Vec<u8>), Error> {
    tokio::task::Builder::new().name(&format!("Downloading chunk {} of {}", self.part_byte, self.file)).spawn(async move {
      let mirror = mirrors.get_mirror_async().await?;
      let mut downloader = download_async::Downloader::new();
      let uri = format!("{}/{}/{}", mirror.base, mirror.version, mirror_path).parse::<download_async::http::Uri>()?;
      warn!("Downloading FilePart: {}", uri);
      downloader.use_uri(uri);
      downloader.use_sockets(mirror.ip);
      downloader.use_progress(progress);
      
      let headers = downloader.headers().expect("Couldn't unwrap download_async headers option");
      headers.append("User-Agent", format!("RenX-Patcher ({})", env!("CARGO_PKG_VERSION")).parse().unwrap());
      headers.append("Range", format!("bytes={}-{}", &self.from, &self.to).parse().unwrap());
    
      let mut buffer = vec![];
      downloader.allow_http();
      let response = downloader.download(download_async::Body::empty(), &mut buffer);
    
      let result = tokio::time::timeout(Duration::from_secs(60), response).await??;
      if result.status != StatusCode::PARTIAL_CONTENT {
        return Err(Error::InvalidStatus(result.status.canonical_reason().unwrap().to_string()))
      }
      Ok((self, buffer))
    })?.await?
  }

  pub(crate) async fn write_to_file(&self, buffer: Vec<u8>) -> Result<(), Error> {
    let from = self.from.clone();
    let file = self.file.clone();
    let part_byte = self.part_byte.clone();

    tokio::task::Builder::new().name(&format!("write chunk {} to {}", part_byte, file)).spawn_blocking(move || {
      let mut f = OpenOptions::new().read(true).write(true).create(true).open(&file)?;
      f.seek(SeekFrom::Start(from))?;
      f.write_all(&buffer)?;
      f.seek(SeekFrom::Start(part_byte))?;
      f.write(&[1])?;
      f.flush()?;
      Ok::<(), Error>(())
    })?.await?
  }
}