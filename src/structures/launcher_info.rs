#[derive(Debug, Clone)]
pub struct LauncherInfo {
  pub version_name: String,
  pub version_number: usize,
  pub patch_url: String,
  pub patch_hash: String,
  pub prompted: bool,
}