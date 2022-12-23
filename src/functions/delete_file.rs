use tracing::instrument;

use crate::Error;

#[instrument]
pub fn delete_file(file: String) -> Result<(), Error> {
    std::fs::remove_file(file)?;
    Ok(())
}