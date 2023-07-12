use std::{fmt::{Formatter, Display}, cell::RefCell};

use async_trait::async_trait;

use super::{abstraction::{ContextAccessor, Worker, Workload, ContextArcM, AppWrapper}, installer::{Product, PackageInstallition}, error::WorkloadError};


pub type UninstallerWrapper = AppWrapper<UninstallerOptions>;

#[derive(Clone)]
pub struct UninstallerOptions {
    pub target_packages: Option<Vec<PackageInstallition>>,
}

impl Default for UninstallerOptions {
    fn default() -> Self {
        UninstallerOptions { target_packages: None }
    }
}

impl Worker for UninstallerWrapper { }

impl ContextAccessor for UninstallerWrapper {
    fn get_context(&self) -> ContextArcM {
        self.app.get_context()
    }

    fn get_product(&self) -> Product {
        self.app.product.clone()
    }
}

#[async_trait] 
impl Workload for UninstallerWrapper {
    async fn run(&self) -> Result<(), WorkloadError> {

        let global = self.app.get_global_script().await?;
        global.if_exist(|s| Ok(s.invoke_before_uninstallition()))?;

            .map_err(|err| WorkloadError::Other(err.to_string()))?
        );  

        let targets = match &self.settings.target_packages {
            Some(opted) => {
                opted.to_owned()
            }
            None => {
                summary_cell.borrow().packages.clone()
            } 
        }; 
        let mut summary = summary_cell.borrow_mut();

        log::info!("Installed packages: {}", summary.packages.iter().map(|e| e.display_name.clone()).collect::<Vec<_>>().join(", ")); 
        log::info!("Packages that will be removed: {}", targets.iter().map(|e| e.display_name.clone()).collect::<Vec<_>>().join(", ")); 
        
        let mut all_done = true;
        for package in targets {
            let remote = repository.get_package(&package.name);
            let script = match remote {
                Some(remote) => { self.app.get_package_script(&remote).wait()? }
                _ => None
            };

            script.if_exist(|s| {
                s.invoke_before_uninstallition();
                Ok(())
            })?;

            log::info!("Starting to delete {} package", package.display_name);
            
            package.files.iter().into_iter().for_each(|file| {
                if let Err(err) = std::fs::remove_file(file.clone()) {
                    log::error!("Failed to delete {:?}. It's included inside {} package. Trace: {}", file.clone(), package.display_name, err);
                    all_done = false;
                } else {
                    log::trace!("Deleted {:?} of {} package.", file.clone(), package.display_name);
                }
            });

            let _ = summary.removed(&package.name).unwrap().save();

            script.if_exist(|s| {
                s.invoke_after_uninstallition();
                Ok(())
            })?;
        };

        if all_done {
            log::info!("Successfully deleted all target packages and their files.");
        }

        if summary.packages.len() == 0 {
            crate::sys::delete_app_entry(&self.get_product())
                .map_err(|err| WorkloadError::Other(format!("Failed to delete app entry. Trace: {}", err)))?;
            log::info!("All packages and their files are deleted. App entry is deleted too.");
        }

        global.if_exist(|s| Ok(s.invoke_after_uninstallition()))?;
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum UninstallerWorkloadState {
    DeletingFiles,
    Interrupted(String),
    Done,
}

unsafe impl Sync for UninstallerWorkloadState {}

impl Display for UninstallerWorkloadState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UninstallerWorkloadState::DeletingFiles => {
                write!(f, "Deleting files")
            },

            UninstallerWorkloadState::Interrupted(err) => {
                write!(f, "Interrupted due to an error. {}", err)
            },

            _ => write!(f, "Done")
        }
    }
}

impl Default for UninstallerWorkloadState {
    fn default() -> Self {
        Self::DeletingFiles
    }
}