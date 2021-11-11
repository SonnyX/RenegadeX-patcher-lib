use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::patcher::Patcher;
use crate::structures::Error;

pub struct PatcherBuilder {
  pub(crate) software_location: String,
  pub(crate) version_url: String
}

impl PatcherBuilder {
    pub fn new() -> Self {
        Self {
            software_location: "".to_string(),
            version_url: "".to_string()
        }
    }

    pub fn set_software_location(&mut self, software_location: String) -> &mut Self {
        self.software_location = software_location;
        self
    }

    pub fn set_version_url(&mut self, version_url: String) -> &mut Self {
        self.version_url = version_url;
        self
    }

    pub fn build(self) -> Result<Patcher, Error> {
        Ok(Patcher {
            in_progress: Arc::new(AtomicBool::new(false)),
            join_handle: None,
            software_location: self.software_location,
            version_url: self.version_url
        })
    }
}