use log::*;

/*
pub trait FileSystem {
  fn get_file_info(&self, file: OsString) -> File;
  fn write_chunk(&self, chunk: Chunk);
  
}
 */






pub trait AsString {
  fn as_string(&self) -> String;
  fn as_string_option(&self) -> Option<String>;
  fn into_inner(self) -> Vec<json::JsonValue>;
}

impl AsString for json::JsonValue {
  fn as_string(&self) -> String {
    match *self {
      json::JsonValue::Short(ref value)  => value.to_string(),
      json::JsonValue::String(ref value) => value.to_string(),
      _                                  => {
        error!("Expected a JSON String");
        panic!("Expected a JSON String")
      }
    }
  }

  fn as_string_option(&self) -> Option<String> {
    match *self {
      json::JsonValue::Short(ref value)  => Some(value.to_string()),
      json::JsonValue::String(ref value) => Some(value.to_string()),
      _                                  => None
    }
  }

  fn into_inner(self) -> Vec<json::JsonValue> {
    match self {
      json::JsonValue::Array(vec) => {
        vec
      },
      _ => vec![]
    }
  }
}

pub trait BorrowUnwrap<T> {
  fn borrow(&self) -> &T;
}

impl<T> BorrowUnwrap<T> for Option<T> {
  fn borrow(&self) -> &T {
    match self {
      Some(val) => val,
      None => {
        error!("called `Option::borrow()` on a `None` value");
        panic!("called `Option::borrow()` on a `None` value")
      },
    }
  }
}

#[derive(Debug)]
pub struct Error {
  inner: Box<dyn std::error::Error + Send + Sync>,
  pub remove_mirror: bool
}

#[derive(Debug)]
pub struct StringError {
  details: String
}

impl std::fmt::Display for StringError {
  #[inline(always)]
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f,"{}", self.details)
  }
}

impl std::error::Error for StringError {}

impl Error {
    #[inline(always)]
    pub fn new(msg: String) -> Error {
      let error = Box::new(StringError { 
        details: msg
    });
      Error {
        inner: error,
        remove_mirror: false
      }
    }
}

impl std::fmt::Display for Error {
  #[inline(always)]
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f,"{}", self.inner)
  }
}


impl std::error::Error for Error {}

impl From<Box<dyn std::error::Error + Send + Sync>> for Error {
  #[inline(always)]
  fn from(error: Box<dyn std::error::Error + Send + Sync>) -> Self {
    Self {
      inner: error,
      remove_mirror: false
    }
  }
}

impl From<std::io::Error> for Error {
  #[inline(always)]
  fn from(error: std::io::Error) -> Self {
    Self {
      inner: Box::new(error),
      remove_mirror: false
    }
  }
}

impl From<tokio::time::error::Elapsed> for Error {
  #[inline(always)]
  fn from(error: tokio::time::error::Elapsed) -> Self {
    Self {
      inner: Box::new(error),
      remove_mirror: true
    }
  }
}

impl From<std::string::FromUtf8Error> for Error {
  #[inline(always)]
  fn from(error: std::string::FromUtf8Error) -> Self {
    Self {
      inner: Box::new(error),
      remove_mirror: false
    }
  }
}

/*
impl From<tokio::timer::timeout::Error<hyper::Error>> for Error {
  #[inline(always)]
  fn from(error: tokio::timer::timeout::Error<hyper::Error>) -> Self {
    use std::error::Error;
    Self {
      details: error.description().to_string(),
      remove_mirror: false
    }
  }
}
*/
impl From<download_async::http::Error> for Error {
  #[inline(always)]
  fn from(error: download_async::http::Error) -> Self {
    error!("http::Error: {:#?}", error);
    Self {
      inner: Box::new(error),
      remove_mirror: false
    }
  }
}

impl From<download_async::http::uri::InvalidUri> for Error {
  #[inline(always)]
  fn from(error: download_async::http::uri::InvalidUri) -> Self {
    error!("http::uri::InvalidUri: {:#?}", error);
    Self {
      inner: Box::new(error),
      remove_mirror: false
    }
  }
}

impl From<std::string::String> for Error {
  #[inline(always)]
  fn from(string: String) -> Self {
    Error {
      inner: Box::new(StringError {
        details: string
      }),
      remove_mirror: false
    }
  }
}

impl From<&str> for Error {
  #[inline(always)]
  fn from(string: &str) -> Self {
    Error {
      inner: Box::new(StringError {
        details: string.to_owned()
      }),
      remove_mirror: true
    }
  }
}

pub trait ExpectUnwrap<T> :  {
  fn unexpected(self, msg: &str) -> T;
}

impl<T, E: std::fmt::Debug> ExpectUnwrap<T> for Result<T, E> {
  #[inline]
  #[track_caller]
  fn unexpected(self, msg: &str) -> T {
    
    match self {
      Ok(val) => val,
      Err(e) => {
        let location = core::panic::Location::caller();
        unwrap_failed(&format!("{}:{}\r\n{}", location.file(), location.line(), msg), &e)
      },
    }
  }
}

impl<T> ExpectUnwrap<T> for Option<T> {
  #[inline]
  #[track_caller]
  fn unexpected(self, msg: &str) -> T {
    match self {
      Some(val) => val,
      None => {
        let location = core::panic::Location::caller();
        expect_failed(&format!("{}:{}\r\n{}", location.file(), location.line(), msg))
      },
    }
  }
}

#[inline(never)]
#[cold]
#[track_caller]
fn expect_failed(msg: &str) -> ! {
  log::error!("{}", msg);
  panic!("{}", msg)
}

#[inline(never)]
#[cold]
#[track_caller]
fn unwrap_failed(msg: &str, error: &dyn std::fmt::Debug) -> ! {
  log::error!("{}: {:?}", msg, error);
  panic!("{}: {:?}", msg, error)
}