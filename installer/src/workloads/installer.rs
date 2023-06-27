
use std::{fmt::{Display, Formatter}};

use super::{abstraction::{Workload, Worker, ContextAccessor}, errors::WorkloadError};
use crate::{InstallerApp, ContextArcT};

use serde::{Deserialize, Serialize};
use async_trait::async_trait;

pub(crate) struct InstallerWrapper {
    app: InstallerApp,
}

impl InstallerWrapper {
    pub fn new(appx: InstallerApp) -> Self {
        InstallerWrapper { app: appx }
    }
}

impl Worker<InstallerWorkloadState> for InstallerWrapper { }

impl ContextAccessor<InstallerWorkloadState> for InstallerWrapper {
    fn get_context(&self) -> ContextArcT<InstallerWorkloadState> {
        self.app.get_context()
    }

    fn get_product(&self) -> Product {
        self.app.product.clone()
    }
}

#[deny(implied_bounds_entailment)]
#[async_trait]
impl Workload<InstallerWorkloadState> for InstallerWrapper {
    async fn run(&self) -> Result<(), WorkloadError> {

        self.get_installition_summary()
            .map_err(|err| WorkloadError::Other(err.to_string()))?;

        log::info!("Starting to install {}", &self.app.product.name);

        self.set_workload_state(InstallerWorkloadState::FetchingRemoteTree(self.app.product.name.clone()));
        
        let repository = self.fetch_repository().await
            .map_err(|err| WorkloadError::Other(err.to_string()))?;

        std::fs::create_dir_all(&self.app.product.target_directory)
            .map_err(|err| WorkloadError::Other(err.to_string()))?;

        for package in &repository.packages {
            log::info!("Downloading the package from {}", &self.app.product.get_uri_to_package(&package));
            self.set_workload_state(InstallerWorkloadState::DownloadingComponent(package.display_name.clone()));

            let package_file = self.get_package(&package).await
                .map_err(|err| WorkloadError::Other(err.to_string()))?;

            log::info!("Decompression of {}", &package.display_name);
            self.set_workload_state(InstallerWorkloadState::InstallingComponent(package.display_name.clone()));

            self.install_package(&package, &package_file).await
                .map_err(|err| WorkloadError::Other(err.to_string()))?;
        }

        self.set_workload_state(InstallerWorkloadState::Done);
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Product {
    pub name: String,
    pub publisher: String,
    pub product_url: String,
    pub control_script: String,
    pub target_directory: String,
    pub repository: String
}

impl Product{
    pub fn get_path_to_package(&self, package: &Package) -> &std::path::Path {
        std::path::Path::new(&self.target_directory)
    }

    pub fn get_uri_to_package(&self, package: &Package) -> String {
        format!("{}packages/{}", self.repository, package.archive)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Repository {
    pub application_name: String,
    pub packages: Vec<Package>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Package {
    pub name: String,
    pub display_name: String,
    pub version: String,
    pub release_date: String,
    pub default: bool,
    pub archive: String,
    pub sha1: String
}

pub struct PackageFile {
    pub handle: tempfile::NamedTempFile,
    pub package: Package
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct InstallitionSummary {
    pub application_name: String,
    pub packages: Vec<PackageInstallition>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct PackageInstallition {
    pub name: String,
    pub display_name: String,
    pub version: String,
    pub installed_at: String,
    pub updated_at: String,
    pub default: bool
}

#[derive(Debug, Clone)]
pub enum InstallerWorkloadState {
    FetchingRemoteTree(String),
    DownloadingComponent(String),
    InstallingComponent(String),
    Interrupted(String),
    Aborted,
    Done,
}

unsafe impl Sync for InstallerWorkloadState {}

impl Display for InstallerWorkloadState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallerWorkloadState::FetchingRemoteTree(str) => {
                write!(f, "Fetching repository: {:?}", str)
            },

            InstallerWorkloadState::DownloadingComponent(str) => {
                write!(f, "Downloading: {:?}", str)
            }, 
            InstallerWorkloadState::InstallingComponent(str) => {
                write!(f, "Installing: {:?}", str)
            },

            InstallerWorkloadState::Done => {
                write!(f, "Done")
            }
            InstallerWorkloadState::Interrupted(str) => {
                write!(f, "Interrupted due error: {}", str)
            },
            InstallerWorkloadState::Aborted => {
                write!(f, "Aborted by user request")
            },

        }
    }
}

impl Default for InstallerWorkloadState {
    fn default() -> Self {
        Self::FetchingRemoteTree("".to_string())
    }
}
