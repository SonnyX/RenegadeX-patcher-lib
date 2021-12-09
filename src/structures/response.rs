/// A Response to a submitted `Request`.
pub struct Response {
  pub parts: Box<download_async::http::response::Parts>,
  pub body: Vec<u8>,
}