use std::fmt::{Formatter, Display};

use async_trait::async_trait;
use definitions::error::PackageUninstallError;
use rust_i18n::error::{Error, ErrorDetails};

use crate::{definitions::{script::ScriptOptional, summary::PackageInstallation}, extensions::future::FutureSyncExt};

use crate::*;
use crate::definitions::context::AppWrapper;
use super::workload::Workload;

pub type UninstallerWrapper = AppWrapper<UninstallerOptions>;

#[derive(Clone)]
pub struct UninstallerOptions {
    pub target_packages: Option<Vec<PackageInstallation>>,
}

impl UninstallerOptions {
    pub fn new(target_packages: Option<Vec<PackageInstallation>>) -> Self {
        UninstallerOptions { target_packages }
    }
}

impl Default for UninstallerOptions {
    fn default() -> Self {
        UninstallerOptions { target_packages: None }
    }
}

#[async_trait] 
impl Workload for UninstallerWrapper {
    async fn run(&mut self) -> Result<(), Error> {
        self.uninstall().wait()?;
        Ok(())
    }
    
    async fn finalize(&mut self, has_error: bool) -> Result<(), Error> {
        let summary = self.app.get_summary();

        // no package is present, full uninstallation
        if summary.get_packages().len() == 0 {
            summary.operations.get_records().into_iter().for_each(|record|{
                if let Err(err) = record.into_operation(None).and_then(|mut operation| operation.revert(&self.app, None)) {
                    log::error!("Failed to revert operation {:?}, global operation. {:?}", record.get_kind(), err);
                }
            }); 
        }

        // all went ok. persist any change has been made.
        if !has_error {
            self.app.persist_summary();
        }

        Ok(())
    }
}

impl UninstallerWrapper {
    pub(self) async fn uninstall(&self) -> Result<(), PackageUninstallError> {
        let global = self.app.download_global_script().await?;
        global.if_exist(|s| Ok(s.invoke_before_uninstallition()?))?;

        let summary = self.app.get_summary(); 
        let targets = match &self.settings.target_packages {
            Some(opted) => {
                opted.to_owned()
            }
            None => {
                summary.packages.clone()
            } 
        }; 

        log::info!("Installed packages: {}", summary.packages.iter().map(|e| e.display_name.clone()).collect::<Vec<_>>().join(", ")); 
        log::info!("Packages that will be removed: {}", targets.iter().map(|e| e.display_name.clone()).collect::<Vec<_>>().join(", "));     

        for package in targets {
            log::info!("Uninstalling, package {}", package.display_name);   
            self.app.set_workload_state(UninstallerWorkloadState::RemovingPackage(package.display_name.clone()));
            self.app.uninstall_package(&package).wait()?;
        };

        global.if_exist(|s| Ok(s.invoke_after_uninstallition()?))?;
        self.app.set_workload_state(UninstallerWorkloadState::Done);
        self.app.set_state_progress(100.0);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum UninstallerWorkloadState {
    RemovingPackage(String),
    Interrupted(ErrorDetails),
    Done,
}

impl Display for UninstallerWorkloadState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UninstallerWorkloadState::RemovingPackage(s) => {
                write!(f, "{}", t!("states.removing-package", [s]))
            },

            UninstallerWorkloadState::Interrupted(e) => {
                write!(f, "{:?}", t!("states.interrupted.by", [e.to_string()]))
            },

            _ => write!(f, "{:?}", t!("states.completed"))
        }
    }
}

impl Default for UninstallerWorkloadState {
    fn default() -> Self {
        Self::Done
    }
}