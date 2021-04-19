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