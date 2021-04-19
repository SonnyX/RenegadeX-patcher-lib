use log::*;

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