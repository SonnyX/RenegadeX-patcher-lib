use crate::structures::NamedUrl;

pub struct SoftwareVersion {
    pub version: String,
    pub version_number: u64,
    pub name: String,
    pub(crate) instructions_hash: String,
    pub(crate) mirrors: Vec<NamedUrl>
}