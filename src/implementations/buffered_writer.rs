use std::io::prelude::*; 
use std::io::{self, SeekFrom};
use crate::structures::{BufWriter, Error};

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
        self.get_mut().flush()?;
        if ret.is_ok() {
          self.written += written as u64;
          (self.callback)(self.inner.as_mut().unwrap(), &mut self.written);
        }
        ret
    }

    pub fn get_mut(&mut self) -> &mut W {
      self.inner.as_mut().unwrap()
    }

    pub fn into_inner(mut self) -> Result<W, Error> {
        self.flush_buf()?;
        Ok(self.inner.take().unwrap())
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
