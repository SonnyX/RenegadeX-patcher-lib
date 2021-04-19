use log::error;

#[derive(Debug)]
pub enum Error {
	InvalidUri(download_async::http::uri::InvalidUri),
	FileLocked(),
	FutureWasPaused(),
	FutureCancelled(),
	HashMismatch(String, String),
	MutexPoisoned(Box<dyn std::fmt::Debug>),
	IoError(std::io::Error),
	NoMirrors(),
	NotUtf8(std::string::FromUtf8Error),
  JsonError(json::Error),

	/// Invalid Json, first argument is the file, second argument is the text of the file
	InvalidJson(String, String),
	OutOfRetries(&'static str),


	// Download related errors:
	HttpError(download_async::http::Error),
	DownloadTimeout(tokio::time::error::Elapsed),
  DownloadError(Box<dyn std::error::Error + Sync + std::marker::Send>),
}

impl std::fmt::Display for Error {
  #[inline(always)]
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f,"{}", self)
  }
}

impl std::error::Error for Error { }


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

impl<T: 'static> From<std::sync::PoisonError<T>> for Error {
  #[inline(always)]
  fn from(error: std::sync::PoisonError<T>) -> Self {
    Self::MutexPoisoned(Box::new(error))
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