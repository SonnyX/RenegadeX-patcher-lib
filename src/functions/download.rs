pub async fn download_file(url: String, timeout: Duration) -> Result<Response, Error> {
    let url : download_async::http::Uri = url.parse::<download_async::http::Uri>()?;

    let req = download_async::http::Request::builder();
    let req = req.uri(url.clone()).header("host", url.host().unwrap()).header("User-Agent", format!("RenX-Patcher ({})", env!("CARGO_PKG_VERSION")));
    let req = req.body(download_async::Body::empty())?;

    let mut buffer = vec![];
    let mut progress : Option<&mut crate::progress::DownloadProgress> = None;

    let response = download_async::download(req, &mut buffer, false, &mut progress, None);
    let result = tokio::time::timeout(timeout, response).await??;
    Ok(Response::new(result, buffer))
}