use crate::patcher::Patcher;

pub struct PatcherBuilder {
  pub(crate) url: string,
  
}

impl PatcherBuilder {
    pub fn new() -> Self {
        Self {
            url: "",

        }
    }

    pub fn build() -> Result<Patcher, Error> {
        
    }
}