use std::fs::OpenOptions;
use sha2::{Sha256, Digest};
use crate::traits::Error;



///
/// Opens a file and calculates it's SHA256 hash
///
pub(crate) fn get_hash(file_path: &str) -> Result<String, Error> {
	let mut file = OpenOptions::new().read(true).open(file_path)?;
	let mut sha256 = Sha256::new();
	std::io::copy(&mut file, &mut sha256)?;
	Ok(hex::encode_upper(sha256.finalize()))
}