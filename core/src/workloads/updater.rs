
use std::fmt::{Formatter, Display};

use self::definitions::summary::PackageInstallation;

use async_trait::async_trait;
use definitions::error::PackageUpdateError;
use rust_i18n::error::{Error, ErrorDetails};

use crate::definitions::script::ScriptOptional;
use crate::extensions::future::FutureSyncExt;
use crate::*;
use crate::definitions::context::AppWrapper;

use super::workload::Workload;

pub type UpdaterWrapper = AppWrapper<UpdaterOptions>;

#[derive(Clone)]
pub struct UpdaterOptions {
    pub target_packages: Option<Vec<PackageInstallation>>,
}

impl UpdaterOptions {
    pub fn new(target_packages: Option<Vec<PackageInstallation>>) -> Self {
        UpdaterOptions { target_packages }
    }
}

impl Default for UpdaterOptions {
    fn default() -> Self {
        UpdaterOptions { target_packages: None }
    }
}

#[async_trait]
impl Workload for UpdaterWrapper {
    async fn run(&mut self) -> Result<(), Error> {
        self.update().wait()?;
        Ok(())
    }

    async fn finalize(&mut self, has_error: bool) -> Result<(), Error> {

        // all went ok. persist any change has been made.
        if !has_error {
            self.app.persist_summary();
        }

        Ok(())
    }
}

impl UpdaterWrapper {
    pub(self) async fn update(&self) -> Result<(), PackageUpdateError> {
        let summary = self.app.get_summary();  

        let global = self.app.download_global_script().await?;
        global.if_exist(|s| Ok(s.invoke_before_update()?))?;

        self.app.set_workload_state(UpdaterWorkloadState::FetchingRemoteTree(self.app.get_product().name.clone()));    
        let repository = self.app.get_repository();

        let state = summary.cross_check(&repository.packages);

        log::info!("Starting to update {}", &self.app.get_product().name);
        log::info!("Installed packages: {}", summary.packages.iter().map(|e| e.display_name.clone()).collect::<Vec<_>>().join(", "));
        log::info!("Packages that are outdated: {}", state.updates.iter().map(|e| e.local.display_name.clone()).collect::<Vec<_>>().join(", "));

        for pair in state.updates {
            let local = pair.local;
            let remote = pair.remote;

            // check if package is opted-out specifically via start args
            if let Some(targets) = &self.settings.target_packages {
                if !targets.iter().any(|f| &f.name == &local.name) {
                    log::info!("Skipping update of {} as it's not listed in target package list. Installed: {}, New: {}.", local.display_name, local.version, remote.version);
                    continue;
                }
            }

            log::info!("Downloading the package file from {}", &self.app.get_product().get_uri_to_package(&remote));
            self.app.set_workload_state(UpdaterWorkloadState::DownloadingComponent(remote.display_name.clone()));
            let update = self.app.download_package(&remote).wait()?;

            log::info!("Removing old installation before update, package {}", &remote.display_name);
            self.app.set_workload_state(UpdaterWorkloadState::RemovingOutdatedComponent(remote.display_name.clone()));
            self.app.uninstall_package(&local).await?;
    
            log::info!("Installing update, package {}", &remote.display_name);
            self.app.set_workload_state(UpdaterWorkloadState::InstallingComponent(remote.display_name.clone()));
            self.app.install_package(&update).await?;
        }

        global.if_exist(|s| Ok(s.invoke_before_update()?))?;

        self.app.set_workload_state(UpdaterWorkloadState::Done);
        self.app.set_state_progress(100.0);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum UpdaterWorkloadState {
    FetchingRemoteTree(String),
    DownloadingComponent(String),
    RemovingOutdatedComponent(String),
    InstallingComponent(String),
    Interrupted(ErrorDetails),
    Aborted,
    Done,
}

impl Display for UpdaterWorkloadState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdaterWorkloadState::FetchingRemoteTree(s) => {
                write!(f, "{:?}", t!("states.fetching-repository", [s]))
            },
            UpdaterWorkloadState::DownloadingComponent(s) => {
                write!(f, "{:?}", t!("states.downloading", [s]))
            },
            UpdaterWorkloadState::RemovingOutdatedComponent(s) => {
                write!(f, "{:?}", t!("states.removing-outdated-package", [s]))
            }, 
            UpdaterWorkloadState::InstallingComponent(s) => {
                write!(f, "{:?}", t!("states.installing", [s]))
            },
            UpdaterWorkloadState::Interrupted(e) => {
                write!(f, "{:?}", t!("states.interrupted.by-error", [e.to_string()]))
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