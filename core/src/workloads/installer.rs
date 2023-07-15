
use std::fmt::{Display, Formatter};

use crate::workloads::error::WorkloadError;

use super::{definitions::*, abstraction::*};

use async_trait::async_trait;

pub type InstallerWrapper = AppWrapper<InstallerOptions>;

#[derive(Clone)]
pub struct InstallerOptions {
    pub target_packages: Option<Vec<Package>>,
}

impl Default for InstallerOptions {
    fn default() -> Self {
        InstallerOptions { target_packages: None }
    }
}

#[async_trait]
impl Workload for InstallerWrapper {
    async fn run(&self) -> Result<(), WorkloadError> {
        log::info!("Starting to install {}", &self.app.get_product().name);

        let global = self.app.get_global_script().await?;
        global.if_exist(|s| Ok(s.invoke_before_installition()))?;

        self.app.set_workload_state(InstallerWorkloadState::FetchingRemoteTree(self.app.get_product().name.clone()));     
        let repository = self.app.fetch_repository().await
            .map_err(|err| WorkloadError::Other(err.to_string()))?;

        std::fs::create_dir_all(&self.app.get_product().target_directory)
            .map_err(|err| WorkloadError::Other(err.to_string()))?;

        // api uses product weak struct, resolves it from filesystem
        self.app.get_product().dump()
            .map_err(|e| WorkloadError::Other(e.to_string()))?;

        let targets = match &self.settings.target_packages {
            None => repository.packages,
            Some(t) => t.to_vec()
        };

        log::info!("Packages in installition queue: {}", targets.iter().map(|e| e.display_name.clone()).collect::<Vec<_>>().join(", "));

        for package in targets {
            if !package.default {
                continue;
            } 
                
            let script = self.app.get_package_script(&package).await?;
            script.if_exist(|s| {
                s.invoke_before_installition();
                Ok(())
            })?;

            log::info!("Starting to install {}, version: {}.", package.display_name, package.version);
            log::info!("Downloading the package file from {}", &self.app.get_product().get_uri_to_package(&package));
            self.app.set_workload_state(InstallerWorkloadState::DownloadingComponent(package.display_name.clone()));

            let package_file = self.app.get_package(&package).await
                .map_err(|err| WorkloadError::Other(err.to_string()))?;

            log::info!("Decompression of {}", &package.display_name);
            self.app.set_workload_state(InstallerWorkloadState::InstallingComponent(package.display_name.clone()));

            self.app.install_package(&package, &package_file)
                .map_err(|err| WorkloadError::Other(err.to_string()))?;

            script.if_exist(|s| {
                s.invoke_after_installition();
                Ok(())
            })?;
        }

        self.app.create_app_entry(&self.app.get_product(), "maintinancetool")
            .map_err(|err| WorkloadError::Other(format!("Failed to create app entry: {}", err.to_string())))?;
        
        self.app.create_maintinance_tool(&self.app.get_product(), "maintinancetool")?;

        global.if_exist(|s| Ok(s.invoke_after_installition()))?;
        self.app.set_workload_state(InstallerWorkloadState::Done);
        self.app.set_state_progress(100.0);
        Ok(())
    }
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

            InstallerWorkloadState::Interrupted(str) => {
                write!(f, "Interrupted due error: {}", str)
            },
            
            InstallerWorkloadState::Aborted => {
                write!(f, "Aborted by user request")
            },
            
            InstallerWorkloadState::Done => {
                write!(f, "Installition is completed")
            }
        }
    }
}

impl Default for InstallerWorkloadState {
    fn default() -> Self {
        Self::FetchingRemoteTree("".to_string())
    }
}