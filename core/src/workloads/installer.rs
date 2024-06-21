
use std::fmt::{Display, Formatter};

use crate::*;
use crate::definitions::script::ScriptOptional;
use crate::extensions::future::FutureSyncExt;
use crate::definitions::context::AppWrapper;
use crate::helpers::file::IoError;

use async_trait::async_trait;
use definitions::error::PackageInstallError;
use rust_i18n::error::{Error, ErrorDetails};

use self::definitions::package::Package;

use super::workload::Workload;

pub type InstallerWrapper = AppWrapper<InstallerOptions>;

#[derive(Clone)]
pub struct InstallerOptions {
    pub target_packages: Option<Vec<Package>>,
}

impl InstallerOptions {
    pub fn new(target_packages: Option<Vec<Package>>) -> Self {
        InstallerOptions { target_packages }
    }
}

impl Default for InstallerOptions {
    fn default() -> Self {
        InstallerOptions { target_packages: None }
    }
}

#[async_trait]
impl Workload for InstallerWrapper {
    async fn run(&mut self) -> Result<(), Error> {
        self.install().wait()?;
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

impl InstallerWrapper {
    pub(self) async fn install(&self) -> Result<(), PackageInstallError> {
        log::info!("Starting to install {}", &self.app.get_product().name);
        log::info!("Target directory {:?}", &self.app.get_product().get_relative_target_directory());

        let global = self.app.download_global_script().await?;
        global.if_exist(|s| Ok(s.invoke_before_installition()?))?;

        std::fs::create_dir_all(&self.app.get_product().get_target_directory())
            .map_err(|err| Error::from(IoError::from(err)))?;

        self.app.dump_product_to_installation_directory(None)?;

        self.app.set_workload_state(InstallerWorkloadState::FetchingRemoteTree(self.app.get_product().name.clone()));     
        let repository = self.app.get_repository();

        let targets = match &self.settings.target_packages {
            None => repository.get_default_packages(),
            Some(t) => t.to_vec()
        };

        log::info!("Packages in installition queue: {}", targets.iter().map(|e| e.display_name.clone()).collect::<Vec<_>>().join(", "));

        for package in targets {  

            log::info!("Starting to install {}, version: {}.", package.display_name, package.version);
            log::info!("Downloading the package file from {}", &self.app.get_product().get_uri_to_package(&package));
            self.app.set_workload_state(InstallerWorkloadState::DownloadingComponent(package.display_name.clone()));
            let package_file = self.app.download_package(&package).await?;

            log::info!("Installing, package {}", &package.display_name);
            self.app.set_workload_state(InstallerWorkloadState::InstallingComponent(package.display_name.clone()));
            self.app.install_package(&package_file).wait()?; // TODO: make err types send
        }

        // means app is doing fresh installition. workload is not invoked by ffi api
        // or via maintinancetool
        if std::env::var("STANDALONE_EXECUTION").is_ok() {
            self.app.create_app_entry("maintenancetool")?;
            self.app.create_maintenance_tool("maintenancetool")?;
        }

        global.if_exist(|s| Ok(s.invoke_after_installition()?))?;
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
    Interrupted(ErrorDetails),
    Aborted,
    Done,
}

impl Display for InstallerWorkloadState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallerWorkloadState::FetchingRemoteTree(s) => {
                write!(f, "{:?}", t!("states.fetching-repository", [s]))
            },

            InstallerWorkloadState::DownloadingComponent(s) => {
                write!(f, "{:?}", t!("states.downloading", [s]))
            }, 
            InstallerWorkloadState::InstallingComponent(s) => {
                write!(f, "{:?}", t!("states.installing", [s]))
            },

            InstallerWorkloadState::Interrupted(e) => {
                write!(f, "{:?}", t!("states.interrupted.by-error", [e.to_string()]))
            },
            
            InstallerWorkloadState::Aborted => {
                write!(f, "{:?}", t!("states.interrupted.by-user"))
            },
            
            InstallerWorkloadState::Done => {
                write!(f, "{:?}", t!("states.completed"))
            }
        }
    }
}

impl Default for InstallerWorkloadState {
    fn default() -> Self {
        Self::FetchingRemoteTree("".to_string())
    }
}