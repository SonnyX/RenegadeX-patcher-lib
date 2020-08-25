use log::*;

//pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

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
  details: String,
  pub remove_mirror: bool
}

impl Error {
    #[inline(always)]
    pub const fn new(msg: String) -> Error {
        Error { 
            details: msg,
            remove_mirror: false
        }
    }
}

impl std::fmt::Display for Error {
  #[inline(always)]
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f,"{}", self.details)
  }
}


impl std::error::Error for Error {}


impl From<std::io::Error> for Error {
  #[inline(always)]
  fn from(error: std::io::Error) -> Self {
    Self {
      details: format!("{}", error),
      remove_mirror: false
    }
  }
}

impl From<tokio::time::Elapsed> for Error {
  #[inline(always)]
  fn from(error: tokio::time::Elapsed) -> Self {
    Self {
      details: format!("{}", error),
      remove_mirror: true
    }
  }
}

impl From<std::string::FromUtf8Error> for Error {
  #[inline(always)]
  fn from(error: std::string::FromUtf8Error) -> Self {
    Self {
      details: format!("{}", error),
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
impl From<http::Error> for Error {
  #[inline(always)]
  fn from(error: http::Error) -> Self {
    error!("http::Error: {:#?}", error);
    Self {
      details: format!("{}", error),
      remove_mirror: false
    }
  }
}

impl From<http::uri::InvalidUri> for Error {
  #[inline(always)]
  fn from(error: http::uri::InvalidUri) -> Self {
    error!("http::uri::InvalidUri: {:#?}", error);
    Self {
      details: format!("{}", error),
      remove_mirror: false
    }
  }
}

impl From<hyper::Error> for Error {
  #[inline(always)]
  fn from(error: hyper::Error) -> Self {
    error!("hyper::Error: {:#?}", error);
    Self {
      details: format!("{}", error),
      remove_mirror: error.is_user()
    }
  }
}

impl From<std::string::String> for Error {
  #[inline(always)]
  fn from(string: String) -> Self {
    Error {
      details: string,
      remove_mirror: false
    }
  }
}

impl From<&str> for Error {
  #[inline(always)]
  fn from(string: &str) -> Self {
    Error {
      details: string.to_string(),
      remove_mirror: true
    }
  }
}

pub trait ExpectUnwrap<T> :  {
  fn unexpected(self, msg: &str) -> T;
}

impl<T, E: std::fmt::Debug> ExpectUnwrap<T> for Result<T, E> {
  #[inline]
  fn unexpected(self, msg: &str) -> T {
    match self {
      Ok(val) => val,
      Err(e) => unwrap_failed(msg, &e),
    }
  }
}

impl<T> ExpectUnwrap<T> for Option<T> {
  #[inline]
  fn unexpected(self, msg: &str) -> T {
    match self {
      Some(val) => val,
      None => expect_failed(msg),
    }
  }
}

#[inline(never)]
#[cold]
fn expect_failed(msg: &str) -> ! {
  log::error!("{}", msg);
  panic!("{}", msg)
}

#[inline(never)]
#[cold]
fn unwrap_failed(msg: &str, error: &dyn std::fmt::Debug) -> ! {
  log::error!("{}: {:?}", msg, error);
  panic!("{}: {:?}", msg, error)
}