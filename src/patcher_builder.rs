use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::patcher::Patcher;
use crate::structures::Error;

pub struct PatcherBuilder {
  pub(crate) game_location: String,
  pub(crate) version_url: String
}

impl PatcherBuilder {
    pub fn new() -> Self {
        Self {
            game_location: "".to_string(),
            version_url: "".to_string()
        }
    }

    pub fn set_game_location(&mut self, game_location: String) -> &mut Self {
        self.game_location = game_location;
        self
    }

    pub fn set_version_url(&mut self, version_url: String) -> &mut Self {
        self.version_url = version_url;
        self
    }

    pub fn build(self) -> Result<Patcher, Error> {
        Ok(Patcher {
            logs: "".to_string(),
            in_progress: Arc::new(AtomicBool::new(false)),
            join_handle: None,
            game_location: self.game_location,
            version_url: self.version_url
        })
    }
}