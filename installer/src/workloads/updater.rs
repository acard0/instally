
use std::{fmt::{Display, Formatter}, cmp::Ordering};

use super::{abstraction::*, errors::*, installer::{Product, PackageInstallition, Package}};
use crate::{ContextArcT, UpdaterApp, helpers::versioning::version_compare};

use async_trait::async_trait;

struct PackagePair {
    local: PackageInstallition,
    remote: Package
}

pub struct UpdaterOptions {
    pub target_packages: Option<Vec<PackageInstallition>>,
}

pub(crate) struct UpdaterAppWrapper {
    app: UpdaterApp,
    opts: UpdaterOptions,
}

impl UpdaterAppWrapper {
    pub fn new(app: UpdaterApp) -> Self {
        UpdaterAppWrapper { app, opts: UpdaterOptions { target_packages: None } }
    }

    pub fn new_with_opts(app: UpdaterApp, opts: UpdaterOptions) -> Self {
        UpdaterAppWrapper { app, opts: opts }
    }
}

impl Worker<UpdaterWorkloadState> for UpdaterAppWrapper { }

impl ContextAccessor<UpdaterWorkloadState> for UpdaterAppWrapper {
    fn get_context(&self) -> ContextArcT<UpdaterWorkloadState> {
        self.app.get_context()
    }

    fn get_product(&self) -> Product {
        self.app.product.clone()
    }
}

#[async_trait]
impl Workload<UpdaterWorkloadState> for UpdaterAppWrapper {

    async fn run(&self) -> Result<(), WorkloadError> {
        let summary = self.get_installition_summary()
            .map_err(|err| WorkloadError::Other(err.to_string()))?;
    
        self.set_workload_state(UpdaterWorkloadState::FetchingRemoteTree(self.app.product.name.clone()));     
        let repository = self.fetch_repository().await
            .map_err(|err| WorkloadError::Other(err.to_string()))?;

        let mut updates = vec![];
        let mut package_map = vec![];

        for remote in repository.packages.iter() {
            match summary.find(remote) {
                Some(local) => {
                    package_map.push( PackagePair { local: local.clone(), remote: remote.clone() } );
        
                    if version_compare(&remote.version, &local.version) == Ordering::Greater{
                        updates.push( PackagePair { local: local.clone(), remote: remote.clone() } );
                    }
                }
                None => { 
                    // package is not installed on local
                }
            }
        }

        log::info!("Starting to update {}", &self.app.product.name);
        log::info!("Installed packages: {}", summary.packages.iter().map(|e| e.display_name.clone()).collect::<Vec<_>>().join(", "));
        log::info!("Packages that are outdated: {}", updates.iter().map(|e| e.local.display_name.clone()).collect::<Vec<_>>().join(", "));

        std::fs::create_dir_all(&self.app.product.target_directory)
            .map_err(|err| WorkloadError::Other(err.to_string()))?;

        for pair in updates {
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