use std::io::prelude::*; 
use std::io::{self, SeekFrom};
use crate::futures::Future;
use futures::future::ok;
use std::time::Duration;
use crate::futures::Stream;
use crate::traits::Error;

/// A Response to a submitted `Request`.
pub struct Response {
    parts: http::response::Parts,
    body: hyper::Chunk,
}
impl Response {
  pub fn new(parts: http::response::Parts, body: hyper::Chunk) -> Self {
    Self {
      parts,
      body
    }
  }

  pub fn headers(&self) -> &http::HeaderMap {
    &self.parts.headers
  }

  pub fn text(&mut self) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(self.body.to_vec())
  }
}

impl AsRef<[u8]> for Response {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.body.as_ref()
    }
}

pub fn download_file(url: String, timeout: Duration) -> Result<Response, Error> {
  if url.contains("http://") {
    let client = hyper::Client::new();
    let url = url.parse::<hyper::Uri>()?;
    let mut req = hyper::Request::builder();
    req.uri(url.clone()).header("host", url.host().unwrap()).header("User-Agent", format!("RenX-Patcher ({})", env!("CARGO_PKG_VERSION")));
    let req = req.body(hyper::Body::empty())?;
    let res = tokio::timer::Timeout::new(client.request(req).and_then(|res| {
      let parts = res.into_parts();
      Future::join(ok::<http::response::Parts, hyper::Error>(parts.0),parts.1.concat2())
    }), timeout);
    let mut rt = tokio::runtime::current_thread::Runtime::new()?;
    let result = rt.block_on(res)?;
    Ok(Response::new(result.0, result.1))
  } else if url.contains("https://") {
    let https = hyper_tls::HttpsConnector::new(4).expect("TLS initialization failed");
    let client = hyper::Client::builder().build::<_, hyper::Body>(https);
    let url = url.parse::<hyper::Uri>()?;
    let mut req = hyper::Request::builder();
    req.uri(url.clone()).header("host", url.host().unwrap()).header("User-Agent", format!("RenX-Patcher ({})", env!("CARGO_PKG_VERSION")));
    let req = req.body(hyper::Body::empty())?;
    let res = tokio::timer::Timeout::new(client.request(req).and_then(|res| {
      let parts = res.into_parts();
      Future::join(ok::<http::response::Parts, hyper::Error>(parts.0),parts.1.concat2())
    }), timeout);
    let mut rt = tokio::runtime::current_thread::Runtime::new()?;
    let result = rt.block_on(res)?;
    Ok(Response::new(result.0, result.1))
  } else {
    Err(Error::new(format!("Unknown file format: {}", url)))
  }
}

pub struct BufWriter<W: Write, F: FnMut(&mut W, &mut u64)> {
    inner: Option<W>,
    buf: Vec<u8>,
    written: u64,
    panicked: bool,
    callback: F,
}

/**
 This should have a buffer that allows for more than 1 MB
 When the buffer reaches 1MB it should write out all data in the buffer to the file, and then clear the buffer.
 
*/

impl<W: Write, F: FnMut(&mut W, &mut u64)> BufWriter<W, F> {
    pub fn new(inner: W, call: F) -> BufWriter<W,F> {
        BufWriter::with_capacity(1_005_000, inner, call)
    }

    pub fn with_capacity(cap: usize, inner: W, call: F) -> BufWriter<W, F> {
        BufWriter {
            inner: Some(inner),
            buf: Vec::with_capacity(cap),
            written: 0,
            panicked: false,
            callback: call,
        }
    }

    fn flush_buf(&mut self) -> io::Result<()> {
        let mut written = 0;
        let len = self.buf.len();
        let mut ret = Ok(());
        while written < len {
            self.panicked = true;
            let r = self.inner.as_mut().unwrap().write(&self.buf[written..]);
            self.panicked = false;

            match r {
                Ok(0) => {
                    ret = Err(std::io::Error::new(std::io::ErrorKind::WriteZero, "failed to write the buffered data"));
                    break;
                }
                Ok(n) => written += n,
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => { ret = Err(e); break }

            }
        }
        if written > 0 {
            self.buf.drain(..written);
        }
        if ret.is_ok() {
          self.written += written as u64;
          (self.callback)(self.inner.as_mut().unwrap(), &mut self.written);
        }
        ret
    }

    pub fn get_mut(&mut self) -> &mut W {
      self.inner.as_mut().unwrap()
    }
}

impl<W: Write, F: FnMut(&mut W, &mut u64)> Write for BufWriter<W, F> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.buf.len() + buf.len() > self.buf.capacity() {
            self.flush_buf()?;
        }
        if buf.len() >= self.buf.capacity() {
            self.panicked = true;
            let r = self.inner.as_mut().unwrap().write(buf);
            self.panicked = false;
            r
        } else {
            self.buf.write(buf)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_buf().and_then(|()| self.get_mut().flush())
    }
}

impl<W: Write, F: FnMut(&mut W, &mut u64)> Drop for BufWriter<W, F> {
    fn drop(&mut self) {
        if self.inner.is_some() && !self.panicked {
            // dtors should not panic, so we ignore a failed flush
            let _r = self.flush_buf();
        }
    }
}

impl<W: Write + Seek, F: FnMut(&mut W, &mut u64)> Seek for BufWriter<W, F> {
    /// Seek to the offset, in bytes, in the underlying writer.
    ///
    /// Seeking always writes out the internal buffer before seeking.
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        if let SeekFrom::Start(posi) = pos {
            self.written = posi;
        };
        self.flush_buf().and_then(|_| self.get_mut().seek(pos))
    }
}
