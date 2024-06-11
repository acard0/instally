use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::helpers::serializer::{self, SerializationError};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Package {
    pub name: String,
    pub display_name: String,
    pub version: String,
    pub release_date: String,
    pub default: bool,
    pub archive: String,
    pub size: u64,
    pub sha1: String,
    pub script: String
}

impl Package {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Package, SerializationError> {
        Ok(serializer::from_json_file(path)?)
    }

    pub fn from_definition(definition: &PackageDefinition, archive: &str, size: u64, sha1: &str, script: &str) -> Package {
        Package {
            name: definition.name.clone(),
            display_name: definition.display_name.clone(),
            version: definition.version.clone(),
            release_date: definition.release_date.clone(),
            default: definition.default,
            archive: archive.to_owned(),
            sha1: sha1.to_owned(),
            script: script.to_owned(),
            size
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct PackageDefinition {
    pub name: String,
    pub display_name: String,
    pub version: String,
    pub release_date: String,
    pub default: bool,
    pub script: String
}

impl PackageDefinition {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<PackageDefinition, SerializationError> {
        Ok(serializer::from_json_file(path)?)
    }

    pub fn define(&self, archive: &str, size: u64, sha1: &str, script: &str) -> Package {
        Package::from_definition(self, archive, size, sha1, script)
    }
}