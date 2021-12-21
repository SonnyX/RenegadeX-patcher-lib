use super::DownloadEntry;

#[derive(Debug)]
pub enum Action {
    Download(DownloadEntry),
    Delete(String),
    Nothing
}