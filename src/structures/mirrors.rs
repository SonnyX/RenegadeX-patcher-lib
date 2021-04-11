use crate::structures::{mirror::Mirror, launcher_info::LauncherInfo};

#[derive(Debug)]
pub struct Mirrors {
  pub mirrors: Vec<Mirror>,
  pub instructions_hash: Option<String>,
  pub version_number: Option<String>,
  pub launcher_info: Option<LauncherInfo>,
}