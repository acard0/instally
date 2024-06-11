
use serde::{Deserialize, Serialize};

use super::package::Package;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Repository {
    pub application_name: String,
    pub script: String,
    pub packages: Vec<Package>,
    pub size: u64
}

impl Repository {
    pub fn new(application_name: &str, size: u64) -> Self {
        Self {
            application_name: application_name.to_string(),
            script: String::new(),
            packages: Vec::new(),
            size
        }
    }

    pub fn get_package(&self, package_name: &str) -> Option<Package> {
        self.packages.iter().find(|e| e.name == package_name)
            .map(|f| f.clone())
    }

    pub fn get_default_packages(&self) -> Vec<Package> {
        self.packages.iter()
            .filter(|e| e.default)
            .cloned()
            .collect()
    }
}