use tokio::task::JoinError;

#[derive(Debug)]
pub enum Error {
	InvalidUri(download_async::http::uri::InvalidUri),
	FileLocked(),
	JoinError(JoinError),
	FutureWasPaused(),
	FutureCancelled(),
	HashMismatch(String, String, String),
	MutexPoisoned(String),
	IoError(std::io::Error),
	NoMirrors(),
	NotUtf8(std::string::FromUtf8Error),
	JsonError(json::Error),

	None(String),
	InvalidServer(),

	/// Invalid Json, first argument is the file, second argument is the text of the file
	InvalidJson(String, String),
	OutOfRetries(&'static str),
	StripPrefix(std::path::StripPrefixError),


	// Download related errors:
	HttpError(download_async::http::Error),
	InvalidStatus(String),
	DownloadTimeout(tokio::time::error::Elapsed),
	DownloadError(Box<dyn std::error::Error + Sync + std::marker::Send>),
	DownloadAsyncError(download_async::Error),
}