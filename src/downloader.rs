use std::io::prelude::*; 
use std::io::{self, Error, ErrorKind, SeekFrom}; 

pub struct BufWriter<W: Write, F: FnMut(&mut W, &mut u64, &mut u64)> {
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

impl<W: Write, F: FnMut(&mut W, &mut u64, &mut u64)> BufWriter<W, F> {
    pub fn new(inner: W, call: F) -> BufWriter<W,F> {
        BufWriter::with_capacity(1005000, inner, call)
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
                    ret = Err(Error::new(ErrorKind::WriteZero, "failed to write the buffered data"));
                    break;
                }
                Ok(n) => written += n,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => { ret = Err(e); break }

            }
        }
        if written > 0 {
            self.buf.drain(..written);
        }
        if ret.is_ok() {
          self.written += written as u64;
          (self.callback)(self.inner.as_mut().unwrap(), &mut self.written, &mut (written as u64));
        }
        ret
    }

    pub fn get_mut(&mut self) -> &mut W {
      self.inner.as_mut().unwrap()
    }
}

impl<W: Write, F: FnMut(&mut W, &mut u64, &mut u64)> Write for BufWriter<W, F> {
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

impl<W: Write, F: FnMut(&mut W, &mut u64, &mut u64)> Drop for BufWriter<W, F> {
    fn drop(&mut self) {
        if self.inner.is_some() && !self.panicked {
            // dtors should not panic, so we ignore a failed flush
            let _r = self.flush_buf();
        }
    }
}

impl<W: Write + Seek, F: FnMut(&mut W, &mut u64, &mut u64)> Seek for BufWriter<W, F> {
    /// Seek to the offset, in bytes, in the underlying writer.
    ///
    /// Seeking always writes out the internal buffer before seeking.
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match pos {
          SeekFrom::Start(posi) => {
            self.written = posi.into();
          },
          _ => {}
        };
        self.flush_buf().and_then(|_| self.get_mut().seek(pos))
    }
}
