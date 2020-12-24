pub enum Error {
	DownloadTimeout(),
	FileLocked(),
	FutureWasPaused(),
	FutureCancelled(),
	HashMismatch(),
}
