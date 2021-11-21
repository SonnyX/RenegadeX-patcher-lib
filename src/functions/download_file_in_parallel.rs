use crate::structures::{Error, Mirrors, Progress};

pub async fn download_file_in_parallel(folder: &str, url: String, mirrors: Mirrors, progress: Progress) -> Result<(), Error> {
    Ok(())
}