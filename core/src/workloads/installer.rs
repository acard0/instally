
use std::fmt::{Display, Formatter};

use crate::{*, error::{Error, ErrorDetails}};

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
    async fn run(&self) -> Result<(), Error> {
        log::info!("Starting to install {}", &self.app.get_product().name);

        let global = self.app.get_global_script().await?;
        global.if_exist(|s| Ok(s.invoke_before_installition()))?;

        self.app.set_workload_state(InstallerWorkloadState::FetchingRemoteTree(self.app.get_product().name.clone()));     
        let repository = self.app.get_repository();

        std::fs::create_dir_all(&self.app.get_product().target_directory)?;

        // api uses product weak struct, resolves it from filesystem
        self.app.get_product().dump()?;

        let targets = match &self.settings.target_packages {
            None => repository.get_default_packages(),
            Some(t) => t.to_vec()
        };

        log::info!("Packages in installition queue: {}", targets.iter().map(|e| e.display_name.clone()).collect::<Vec<_>>().join(", "));

        for package in targets {  
            let script = self.app.get_package_script(&package).await?;
            script.if_exist(|s| {
                s.invoke_before_installition();
                Ok(())
            })?;

            log::info!("Starting to install {}, version: {}.", package.display_name, package.version);
            log::info!("Downloading the package file from {}", &self.app.get_product().get_uri_to_package(&package));
            self.app.set_workload_state(InstallerWorkloadState::DownloadingComponent(package.display_name.clone()));

            let package_file = self.app.get_package(&package).await?;

            log::info!("Decompression of {}", &package.display_name);
            self.app.set_workload_state(InstallerWorkloadState::InstallingComponent(package.display_name.clone()));

            self.app.install_package(&package, &package_file)?;

            script.if_exist(|s| {
                s.invoke_after_installition();
                Ok(())
            })?;
        }

        self.app.create_app_entry(&self.app, "maintenancetool")?;
        
        if std::env::var("STANDALONE_EXECUTION").is_ok() {
            self.app.create_maintenance_tool(&self.app, "maintenancetool")?;
        }

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
    Interrupted(ErrorDetails),
    Aborted,
    Done,
}

unsafe impl Sync for InstallerWorkloadState {}

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