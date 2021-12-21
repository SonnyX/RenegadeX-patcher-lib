
#[derive(Debug)]
pub enum Action {
    DownloadFull,
    DownloadDelta,
    Delete,
    Nothing
}