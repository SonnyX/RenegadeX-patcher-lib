use crate::structures::Response;

impl Response {
  pub fn new(parts: download_async::http::response::Parts, body: Vec<u8>) -> Self {
    Self {
      parts,
      body
    }
  }

  pub fn headers(&self) -> &download_async::http::HeaderMap {
    &self.parts.headers
  }

  pub fn text(&mut self) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(self.body.clone())
  }
}

impl AsRef<[u8]> for Response {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.body.as_ref()
    }
}