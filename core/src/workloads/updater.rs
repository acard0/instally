
use std::{fmt::{Display, Formatter}};

use super::{abstraction::*, errors::*, installer::{Product, PackageInstallition, Package}};

use async_trait::async_trait;

pub struct PackagePair {
    pub local: PackageInstallition,
    pub remote: Package
}

pub struct CrossCheckSummary {
    pub map: Vec<PackagePair>,
    pub updates: Vec<PackagePair>
}

pub struct UpdaterOptions {
    pub target_packages: Option<Vec<PackageInstallition>>,
}

pub struct UpdaterAppWrapper {
    app: InstallyApp,
    opts: UpdaterOptions,
}

impl UpdaterAppWrapper {
    pub fn new(app: InstallyApp) -> Self {
        UpdaterAppWrapper { app, opts: UpdaterOptions { target_packages: None } }
    }

    pub fn new_with_opts(app: InstallyApp, opts: UpdaterOptions) -> Self {
        UpdaterAppWrapper { app, opts: opts }
    }
}

impl Worker for UpdaterAppWrapper { }

impl ContextAccessor for UpdaterAppWrapper {
    fn get_context(&self) -> ContextArcM {
        self.app.get_context()
    }

    fn get_product(&self) -> Product {
        self.app.product.clone()
    }
}

#[async_trait]
impl Workload for UpdaterAppWrapper {
    async fn run(&self) -> Result<(), WorkloadError> {
        self.set_workload_state(UpdaterWorkloadState::FetchingRemoteTree(self.app.product.name.clone()));  

        let summary = self.get_installition_summary()
            .map_err(|err| WorkloadError::Other(err.to_string()))?;
    
        let repository = self.fetch_repository().await
            .map_err(|err| WorkloadError::Other(err.to_string()))?;

        let state = summary.cross_check(&repository.packages).await
            .map_err(|err| WorkloadError::Other(err.to_string()))?;

        log::info!("Starting to update {}", &self.app.product.name);
        log::info!("Installed packages: {}", summary.packages.iter().map(|e| e.display_name.clone()).collect::<Vec<_>>().join(", "));
        log::info!("Packages that are outdated: {}", state.updates.iter().map(|e| e.local.display_name.clone()).collect::<Vec<_>>().join(", "));

        for pair in state.updates {
            let local = pair.local;
            let remote = pair.remote;
            
            // check if package is opted-out specifically via start args
            if let Some(targets) = &self.opts.target_packages {
                if !targets.iter().any(|f| &f.name == &local.name) {
                    log::info!("Skipping update of {} as it's not listed in target package list. Installed: {}, New: {}.", local.display_name, local.version, remote.version);
                    continue;
                }
            }

            log::info!("Starting to update {}, installed: {}, new: {}.", local.display_name, local.version, remote.version);
            log::info!("Downloading the package file from {}", &self.app.product.get_uri_to_package(&remote));
            self.set_workload_state(UpdaterWorkloadState::DownloadingComponent(remote.display_name.clone()));

            let package_file = self.get_package(&remote).await
                .map_err(|err| WorkloadError::Other(err.to_string()))?;

            log::info!("Decompression of {}", &remote.display_name);
            self.set_workload_state(UpdaterWorkloadState::InstallingComponent(remote.display_name.clone()));

            self.install_package(&remote, &package_file).await
                .map_err(|err| WorkloadError::Other(err.to_string()))?;
        }

        self.set_workload_state(UpdaterWorkloadState::Done);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum UpdaterWorkloadState {
    FetchingRemoteTree(String),
    DownloadingComponent(String),
    InstallingComponent(String),
    Interrupted(String),
    Aborted,
    Done,
}

unsafe impl Sync for UpdaterWorkloadState {}

impl Display for UpdaterWorkloadState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdaterWorkloadState::FetchingRemoteTree(str) => {
                write!(f, "Fetching repository: {:?}", str)
            },

            UpdaterWorkloadState::DownloadingComponent(str) => {
                write!(f, "Downloading: {:?}", str)
            }, 
            UpdaterWorkloadState::InstallingComponent(str) => {
                write!(f, "Installing: {:?}", str)
            },

            UpdaterWorkloadState::Interrupted(str) => {
                write!(f, "Interrupted due error: {}", str)
            },
            
            UpdaterWorkloadState::Aborted => {
                write!(f, "Aborted by user request")
            },
            
            UpdaterWorkloadState::Done => {
                write!(f, "Update is completed")
            }
        }
    }
}

impl Default for UpdaterWorkloadState {
    fn default() -> Self {
        Self::FetchingRemoteTree("".to_string())
    }
}