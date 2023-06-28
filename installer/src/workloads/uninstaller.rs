use std::{fmt::{Formatter, Display}};

use async_trait::async_trait;

use crate::{ContextArcT, UninstallerApp};

use super::{abstraction::{ContextAccessor, Worker, Workload}, installer::Product, errors::WorkloadError};


pub(crate) struct UninstallerWrapper {
    app: UninstallerApp,
}

impl UninstallerWrapper {
    pub fn new(appx: UninstallerApp) -> Self {
        UninstallerWrapper { app: appx }
    }
}

impl Worker<UninstallerWorkloadState> for UninstallerWrapper { }

impl ContextAccessor<UninstallerWorkloadState> for UninstallerWrapper {
    fn get_context(&self) -> ContextArcT<UninstallerWorkloadState> {
        self.app.get_context()
    }

    fn get_product(&self) -> Product {
        self.app.product.clone()
    }
}

#[async_trait]
impl Workload<UninstallerWorkloadState> for UninstallerWrapper {
    async fn run(&self) -> Result<(), WorkloadError> {
        let mut summary = self.get_installition_summary()
            .map_err(|err| WorkloadError::Other(err.to_string()))?;
        
        log::info!("Installed packages: {}", summary.packages.iter().map(|e| e.display_name.clone()).collect::<Vec<_>>().join(", ")); 
        
        let mut all_done = true;
        for package in summary.clone().packages.into_iter() {
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