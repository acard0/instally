use core::time;
use std::{thread, fmt::{Display, Formatter}};

use super::abstraction::{Workload, WorkloadResult, Worker, ContextAccessor};
use crate::{InstallerApp, ContextArcT};

use serde::{Deserialize, Serialize};
use serde_xml_rs::{from_str};
use async_trait::async_trait;

pub(crate) struct InstallerWrapper {
    app: InstallerApp,
}

impl InstallerWrapper {
    pub fn new(appx: InstallerApp) -> Self {
        InstallerWrapper { app: appx }
    }
}

impl Worker<InstallerWorkloadState> for InstallerWrapper { }

impl ContextAccessor<InstallerWorkloadState> for InstallerWrapper {
    fn get_context(&self) -> ContextArcT<InstallerWorkloadState> {
        self.app.get_context()
    }
}

#[async_trait]
impl Workload<InstallerWorkloadState> for InstallerWrapper {
    async fn run(&self) -> WorkloadResult {
        
        //TODO

        self.set_workload_state(InstallerWorkloadState::Done);
        WorkloadResult::Ok
    }
    
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Repository {
    pub application_name: String,
    pub application_version: String,
    pub packages: Vec<PackageUpdate>
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct PackageUpdate {
    pub name: String,
    pub display_name: String,
    pub version: String,
    pub release_date: String,
    pub default: bool,
    pub downloadable_archives: String,
    pub sha1: String
}

#[derive(Debug, Clone)]
pub enum InstallerWorkloadState {
    FetchingRemoteTree(String),
    DownloadingComponent(String),
    InstallingComponent(String),
    Done,
}

unsafe impl Sync for InstallerWorkloadState {}

impl Display for InstallerWorkloadState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallerWorkloadState::FetchingRemoteTree(str) => {
                write!(f, "Fetching repository: {:?}", str)
            },

            InstallerWorkloadState::DownloadingComponent(str) => {
                write!(f, "Downloading: {:?}", str)
            }, 
            InstallerWorkloadState::InstallingComponent(str) => {
                write!(f, "Installing: {:?}", str)
            },

            InstallerWorkloadState::Done => {
                write!(f, "Done")
            }

        }
    }
}

impl Default for InstallerWorkloadState {
    fn default() -> Self {
        Self::FetchingRemoteTree("".to_string())
    }
}
