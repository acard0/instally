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
        let summary_cell = RefCell::new(self.get_installition_summary_target()
            .map_err(|err| WorkloadError::Other(err.to_string()))?
        );  

        let targets = match &self.settings.target_packages {
            Some(opted) => {
                opted.to_owned()
            }
            None => {
                summary_cell.borrow().packages.clone()   // Immutable borrow
            } 
        }; 
        let mut summary = summary_cell.borrow_mut();

        log::info!("Installed packages: {}", summary.packages.iter().map(|e| e.display_name.clone()).collect::<Vec<_>>().join(", ")); 
        log::info!("Packages that will be removed: {}", targets.iter().map(|e| e.display_name.clone()).collect::<Vec<_>>().join(", ")); 
        
        let mut all_done = true;
        for package in targets {

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
        };

        if all_done {
            log::info!("Successfully deleted all packages and their files.");
        }

        self.set_workload_state(UninstallerWorkloadState::Done);
        self.set_state_progress(100.0);
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