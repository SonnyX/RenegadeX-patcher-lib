use crate::Error;

pub fn restore_backup(path: &str) -> Result<(), Error> {
    std::fs::remove_file(path)?;
    std::fs::rename(format!("{}.bck", path), path)?;
    Ok(())
}