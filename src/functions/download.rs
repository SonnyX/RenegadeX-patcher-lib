use std::time::Duration;
use crate::structures::Error;
use crate::structures::Response;
use download_async;

pub async fn download_file(url: String, timeout: Duration) -> Result<Response, Error> {
    let mut downloader = download_async::Downloader::new();
    downloader.use_uri(url.parse::<download_async::http::Uri>()?);
    let headers = downloader.headers().expect("Couldn't unwrap download_async headers option");
    headers.append("User-Agent", format!("RenX-Patcher ({})", env!("CARGO_PKG_VERSION")).parse().unwrap());
    let mut buffer = vec![];
    downloader.allow_http();
    let response = downloader.download(download_async::Body::empty(), &mut buffer);

    let result = tokio::time::timeout(timeout, response).await??;
    Ok(Response::new(result, buffer))
}