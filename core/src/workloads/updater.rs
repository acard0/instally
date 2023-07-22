
use std::fmt::{Formatter, Display};

use super::{definitions::*, abstraction::*};

use async_trait::async_trait;

use crate::{*, error::Error};

pub type UpdaterWrapper = AppWrapper<UpdaterOptions>;

#[derive(Clone)]
pub struct UpdaterOptions {
    pub target_packages: Option<Vec<PackageInstallition>>,
}

impl Default for UpdaterOptions {
    fn default() -> Self {
        UpdaterOptions { target_packages: None }
    }
}

#[async_trait]
impl Workload for UpdaterWrapper {
    async fn run(&self) -> Result<(), Error> {
        let global = self.app.get_global_script().await?;
        global.if_exist(|s| Ok(s.invoke_before_update()))?;

        self.app.set_workload_state(UpdaterWorkloadState::FetchingRemoteTree(self.app.get_product().name.clone()));  

        let summary = self.app.get_installition_summary_target()?;
    
        let repository = self.app.get_repository();

        let state = summary.cross_check(&repository.packages)?;

        log::info!("Starting to update {}", &self.app.get_product().name);
        log::info!("Installed packages: {}", summary.packages.iter().map(|e| e.display_name.clone()).collect::<Vec<_>>().join(", "));
        log::info!("Packages that are outdated: {}", state.updates.iter().map(|e| e.local.display_name.clone()).collect::<Vec<_>>().join(", "));

        for pair in state.updates {
            let local = pair.local;
            let remote = pair.remote;

            let script = self.app.get_package_script(&remote).await?;

            script.if_exist(|s| {
                s.invoke_before_installition();
                Ok(())
            })?;
            
            // check if package is opted-out specifically via start args
            if let Some(targets) = &self.settings.target_packages {
                if !targets.iter().any(|f| &f.name == &local.name) {
                    log::info!("Skipping update of {} as it's not listed in target package list. Installed: {}, New: {}.", local.display_name, local.version, remote.version);
                    continue;
                }
            }

            log::info!("Starting to update {}, installed: {}, new: {}.", local.display_name, local.version, remote.version);
            log::info!("Downloading the package file from {}", &self.app.get_product().get_uri_to_package(&remote));
            self.app.set_workload_state(UpdaterWorkloadState::DownloadingComponent(remote.display_name.clone()));

            let package_file = self.app.get_package(&remote).await?;

            log::info!("Decompression of {}", &remote.display_name);
            self.app.set_workload_state(UpdaterWorkloadState::InstallingComponent(remote.display_name.clone()));

            self.app.install_package(&remote, &package_file)?;

            script.if_exist(|s| {
                s.invoke_after_update();
                Ok(())
            })?;
        }

        global.if_exist(|s| Ok(s.invoke_before_update()))?;

        self.app.set_workload_state(UpdaterWorkloadState::Done);
        self.app.set_state_progress(100.0);
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
            UpdaterWorkloadState::FetchingRemoteTree(s) => {
                write!(f, "{:?}", t!("states.fetching-repository", [s]))
            },

            UpdaterWorkloadState::DownloadingComponent(s) => {
                write!(f, "{:?}", t!("states.downloading", [s]))
            }, 
            UpdaterWorkloadState::InstallingComponent(s) => {
                write!(f, "{:?}", t!("states.installing", [s]))
            },

            UpdaterWorkloadState::Interrupted(s) => {
                write!(f, "{:?}", t!("states.interrupted.by-error", [s]))
            },
            
            UpdaterWorkloadState::Aborted => {
                write!(f, "{:?}", t!("states.interrupted.by-user"))
            },
            
            UpdaterWorkloadState::Done => {
                write!(f, "{:?}", t!("states.completed"))
            }
        }
    }
}

impl Default for UpdaterWorkloadState {
    fn default() -> Self {
        Self::FetchingRemoteTree("".to_string())
    }
}