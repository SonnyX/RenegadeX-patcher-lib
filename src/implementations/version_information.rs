use std::time::Duration;

use crate::functions::download_file;
use crate::structures::{Error, NamedUrl, VersionInformation, LauncherVersion, SoftwareVersion};

use crate::traits::AsString;

impl VersionInformation {
    pub async fn retrieve(url: &str) -> Result<Self, Error> {
        let mut response = download_file(url.to_string(), Duration::from_secs(10)).await?;
        let file = response.text()?;
        let parsed_json = json::parse(&file)?;
        let mirrors : Vec<NamedUrl> = parsed_json["game"]["mirrors"].members().map(|json| NamedUrl {
            name: json["name"].as_string(),
            url: json["url"].as_string(),
        }).collect();
        Ok(Self {
            launcher: LauncherVersion {
                version: parsed_json["launcher"]["version_name"].as_string(),
                url: parsed_json["launcher"]["patch_url"].as_string(),
                hash: parsed_json["launcher"]["patch_hash"].as_string(),
            },
            software: SoftwareVersion {
                name: parsed_json["game"]["version_name"].as_string(),
                version: parsed_json["game"]["patch_path"].as_string(),
                version_number: parsed_json["game"]["version_number"].as_u64().ok_or::<Error>(Error::None(format!("Cannot parse \"{}\" as u64", parsed_json["game"]["version_number"].dump())))?,
                instructions_hash: parsed_json["game"]["instructions_hash"].as_string(),
                mirrors
            }
        })
    }
}