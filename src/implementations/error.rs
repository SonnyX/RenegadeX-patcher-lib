use crate::structures::Error;
use log::error;

impl std::error::Error for Error { }

impl std::fmt::Display for Error {
  #[inline(always)]
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f,"{}", self)
  }
}

impl From<std::path::StripPrefixError> for Error {
  #[inline(always)]
  fn from(error: std::path::StripPrefixError) -> Self {
    error!("std::path::StripPrefixError: {:#?}", error);
    Self::StripPrefix(error)
  }
}

impl From<download_async::http::uri::InvalidUri> for Error {
  #[inline(always)]
  fn from(error: download_async::http::uri::InvalidUri) -> Self {
    error!("http::uri::InvalidUri: {:#?}", error);
    Self::InvalidUri(error)
  }
}

impl From<download_async::http::Error> for Error {
  #[inline(always)]
  fn from(error: download_async::http::Error) -> Self {
    error!("http::Error: {:#?}", error);
    Self::HttpError(error)
  }
}


impl From<download_async::Error> for Error {
  #[inline(always)]
  fn from(error: download_async::Error) -> Self {
    error!("download_async::Error: {:#?}", error);
    Self::DownloadAsyncError(error)
  }
}

impl<T> From<std::sync::PoisonError<std::sync::MutexGuard<'_, T>>> for Error {
  #[inline(always)]
  fn from(error: std::sync::PoisonError<std::sync::MutexGuard<'_, T>>) -> Self {
    use std::error::Error;
    let error = error.source().unwrap();
    Self::MutexPoisoned(error.to_string())
  }
}

impl From<tokio::time::error::Elapsed> for Error {
  #[inline(always)]
  fn from(error: tokio::time::error::Elapsed) -> Self {
    Self::DownloadTimeout(error)
  }
}

impl From<std::io::Error> for Error {
  #[inline(always)]
  fn from(error: std::io::Error) -> Self {
    Self::IoError(error)
  }
}

impl From<std::string::FromUtf8Error> for Error {
  #[inline(always)]
  fn from(error: std::string::FromUtf8Error) -> Self {
    Self::NotUtf8(error)
  }
}

impl From<Box<dyn std::error::Error + Sync + std::marker::Send>> for Error {
  #[inline(always)]
  fn from(error: Box<dyn std::error::Error + Sync + std::marker::Send>) -> Self {
    Self::DownloadError(error)
  }
}

impl From<json::Error> for Error {
  #[inline(always)]
  fn from(error: json::Error) -> Self {
    Self::JsonError(error)
  }
}