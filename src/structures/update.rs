#[derive(Debug, Clone)]
pub enum Update {
  Unknown,
  UpToDate,
  Resume,
  Full,
  Delta,
}