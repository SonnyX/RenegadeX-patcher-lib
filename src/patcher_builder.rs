use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::{NamedUrl, Progress};
use crate::patcher::Patcher;
use crate::structures::{Error, Mirrors};

pub struct PatcherBuilder {
  pub(crate) software_location: Option<String>,
  pub(crate) mirrors: Option<Vec<NamedUrl>>,
  pub(crate) version: Option<String>,
  pub(crate) instructions_hash: Option<String>,
}

impl PatcherBuilder {
    pub fn new() -> Self {
        Self {
            software_location: None,
            mirrors: None,
            version: None,
            instructions_hash: None,
        }
    }

    pub fn set_software_location(&mut self, software_location: String) -> &mut Self {
        self.software_location = Some(software_location);
        self
    }

    
    pub fn set_software_information(&mut self, mirrors: Vec<NamedUrl>, version: String, instructions_hash: String) -> &mut Self {
        self.mirrors = Some(mirrors);
        self.version = Some(version);
        self.instructions_hash = Some(instructions_hash);
        self
    }

    pub fn set_success_callback<F>(&mut self, func: F) -> &mut Self 
        where F: Fn()
    {
        self
    }

    pub fn set_failure_callback<F>(&mut self, func: F) -> &mut Self 
        where F: Fn(Error)
    {
        self
    }

    pub fn set_progress_callback<F>(&mut self, func: F) -> &mut Self 
        where F: Fn(Progress)
    {
        self
    }

    pub fn build(self) -> Result<Patcher, Error> {

        Ok(Patcher {
            in_progress: Arc::new(AtomicBool::new(false)),
            join_handle: None,
            software_location: self.software_location.expect(""),
            mirrors: Mirrors::new(self.mirrors.expect(""), self.version.expect("")),
            instructions_hash: self.instructions_hash.expect("")
        })
    }
}