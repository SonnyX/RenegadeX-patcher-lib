use std::{fs::OpenOptions, io::Read};
use sha2::{Sha256, Digest};
use crate::structures::Error;

/// Opens a file and calculates it's SHA256 hash
pub(crate) fn get_hash(file_path: &str) -> Result<String, Error> {
	let mut file = OpenOptions::new().read(true).open(file_path)?;
	let mut hasher = Sha256::new();
	let mut read : usize;
	let mut buffer = [0u8; 4096];
	while (read = file.read(&mut buffer)?, read != 0).1 {
		hasher.update(&buffer[..read]);
	}
	drop(file);
	drop(buffer);
	Ok(hex::encode_upper(hasher.finalize()))
}