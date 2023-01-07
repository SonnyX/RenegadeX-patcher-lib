use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::pausable::FutureContext;
use crate::{NamedUrl, Progress};
use crate::patcher::Patcher;
use crate::structures::{Error, Mirrors};

pub struct PatcherBuilder {
  pub(crate) software_location: Option<String>,
  pub(crate) mirrors: Option<Vec<NamedUrl>>,
  pub(crate) version: Option<String>,
  pub(crate) instructions_hash: Option<String>,
  pub(crate) success_callback: Option<Box<dyn FnOnce() + Send>>,
  pub(crate) failure_callback: Option<Box<dyn FnOnce(Error) + Send>>,
  pub(crate) progress_callback: Option<Box<dyn Fn(&Progress) + Send>>,
}

impl PatcherBuilder {
    pub fn new() -> Self {
        Self {
            software_location: None,
            mirrors: None,
            version: None,
            instructions_hash: None,
            success_callback: None,
            failure_callback: None,
            progress_callback: None
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

    pub fn set_success_callback(&mut self, func: Box<dyn FnOnce() + Send>) -> &mut Self 
    {
        self.success_callback = Some(func);
        self
    }

    pub fn set_failure_callback(&mut self, func: Box<dyn FnOnce(Error) + Send>) -> &mut Self 
    {
        self.failure_callback = Some(func);
        self
    }

    pub fn set_progress_callback(&mut self, func: Box<dyn Fn(&Progress) + Send>) -> &mut Self 
    {
        self.progress_callback = Some(func);
        self
    }

    pub fn build(self) -> Result<Patcher, Error> {

        Ok(Patcher {
            in_progress: Arc::new(AtomicBool::new(false)),
            join_handle: None,
            software_location: self.software_location.expect(""),
            mirrors: Mirrors::new(self.mirrors.expect(""), self.version.expect("")),
            instructions_hash: self.instructions_hash.expect(""),
            success_callback: self.success_callback,
            failure_callback: self.failure_callback,
            progress_callback: self.progress_callback,
            context: Arc::new(FutureContext::new())
        })
    }
}