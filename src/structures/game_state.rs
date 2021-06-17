#[derive(Debug, Clone)]
pub enum GameState {
  Unknown,
  UpToDate,
  Resume,
  Full,
  Delta,
}