use crate::structures::Error;

impl std::error::Error for Error { }

impl std::fmt::Display for Error {
  #[track_caller]
  #[inline(always)]
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f,"{}", self)
  }
}

impl From<std::path::StripPrefixError> for Error {
  #[track_caller]
  #[inline(always)]
  fn from(error: std::path::StripPrefixError) -> Self {
    log_error(&error);
    Self::StripPrefix(error)
  }
}

impl From<tokio::task::JoinError> for Error {
  #[track_caller]
  #[inline(always)]
  fn from(error: tokio::task::JoinError) -> Self {
    log_error(&error);
    Self::JoinError(error)
  }
}

impl From<download_async::http::uri::InvalidUri> for Error {
  #[track_caller]
  #[inline(always)]
  fn from(error: download_async::http::uri::InvalidUri) -> Self {
    log_error(&error);
    Self::InvalidUri(error)
  }
}

impl From<download_async::http::Error> for Error {
  #[track_caller]
  #[inline(always)]
  fn from(error: download_async::http::Error) -> Self {
    log_error(&error);
    Self::HttpError(error)
  }
}


impl From<download_async::Error> for Error {
  #[track_caller]
  #[inline(always)]
  fn from(error: download_async::Error) -> Self {
    log_error(&error);
    Self::DownloadAsyncError(error)
  }
}

impl<T> From<std::sync::PoisonError<std::sync::MutexGuard<'_, T>>> for Error {
  #[track_caller]
  #[inline(always)]
  fn from(error: std::sync::PoisonError<std::sync::MutexGuard<'_, T>>) -> Self {
    use std::error::Error;
    log_error(&error);
    let error = error.source().unwrap();
    log_error(&error);
    Self::MutexPoisoned(error.to_string())
  }
}

impl From<tokio::time::error::Elapsed> for Error {
  #[track_caller]
  #[inline(always)]
  fn from(error: tokio::time::error::Elapsed) -> Self {
    log_error(&error);
    Self::DownloadTimeout(error)
  }
}

impl From<std::io::Error> for Error {
  #[track_caller]
  #[inline(always)]
  fn from(error: std::io::Error) -> Self {
    log_error(&error);
    Self::IoError(error)
  }
}

impl From<std::string::FromUtf8Error> for Error {
  #[track_caller]
  #[inline(always)]
  fn from(error: std::string::FromUtf8Error) -> Self {
    log_error(&error);
    Self::NotUtf8(error)
  }
}

impl From<Box<dyn std::error::Error + Sync + std::marker::Send>> for Error {
  #[track_caller]
  #[inline(always)]
  fn from(error: Box<dyn std::error::Error + Sync + std::marker::Send>) -> Self {
    log_error(&error.source().unwrap());
    Self::DownloadError(error)
  }
}

impl From<json::Error> for Error {
  #[track_caller]
  #[inline(always)]
  fn from(error: json::Error) -> Self {
    log_error(&error);
    Self::JsonError(error)
  }
}

#[track_caller]
fn log_error(error: &(impl std::error::Error + ?Sized)) {
  tracing::error!("{:?}", error);
}