use crate::structures::{Error, Progress};

impl Progress {
    pub fn get_current_action(&self) -> Result<String, Error> {
        Ok((*self.current_action.lock()?).clone())
    }

    pub(crate) fn set_current_action(&self, value: String) -> Result<(), Error> {
        *self.current_action.lock()? = value;
        Ok(())
    }
}