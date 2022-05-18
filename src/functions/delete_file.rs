use crate::Error;

pub async fn delete_file(file: String) -> Result<(), Error> {
    std::fs::remove_file(file)?;
    Ok(())
}