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
      _                                  => panic!("Expected a JSON String")
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
      None => panic!("called `Option::borrow()` on a `None` value"),
    }
  }
}

#[derive(Debug)]
pub struct Error {
  details: String
}

impl Error {
    pub const fn new(msg: String) -> Error {
        Error { details: msg }
    }
}

impl std::error::Error for Error {
  fn description(&self) -> &str {
    &self.details
  }
}

impl std::fmt::Display for Error {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f,"{}", self.details)
  }
}

impl From<reqwest::Error> for Error {
  fn from(error: reqwest::Error) -> Self {
    use std::error::Error;
    Self {
      details: error.description().to_string()
    }
  }
}

impl From<std::string::String> for Error {
  fn from(string: String) -> Self {
    Error {
      details: string
    }
  }
}

