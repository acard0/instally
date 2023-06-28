
use std::{fmt::{Display, Formatter}, ops::{Deref, DerefMut}, io::{Read, Write, Seek}, path::PathBuf};

use super::{abstraction::{Workload, Worker, ContextAccessor}, errors::{WorkloadError, WeakStructParseError, PackageUninstallError}};
use crate::{InstallerApp, ContextArcT};

use serde::{Deserialize, Serialize};
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

    fn get_product(&self) -> Product {
        self.app.product.clone()
    }
}

#[async_trait]
impl Workload<InstallerWorkloadState> for InstallerWrapper {
    async fn run(&self) -> Result<(), WorkloadError> {
        log::info!("Starting to install {}", &self.app.product.name);

        self.set_workload_state(InstallerWorkloadState::FetchingRemoteTree(self.app.product.name.clone()));     
        let repository = self.fetch_repository().await
            .map_err(|err| WorkloadError::Other(err.to_string()))?;

        std::fs::create_dir_all(&self.app.product.target_directory)
            .map_err(|err| WorkloadError::Other(err.to_string()))?;

        for package in &repository.packages {
            log::info!("Starting to install {}, version: {}.", package.display_name, package.version);
            log::info!("Downloading the package file from {}", &self.app.product.get_uri_to_package(&package));
            self.set_workload_state(InstallerWorkloadState::DownloadingComponent(package.display_name.clone()));

            let package_file = self.get_package(&package).await
                .map_err(|err| WorkloadError::Other(err.to_string()))?;

            log::info!("Decompression of {}", &package.display_name);
            self.set_workload_state(InstallerWorkloadState::InstallingComponent(package.display_name.clone()));

            self.install_package(&package, &package_file).await
                .map_err(|err| WorkloadError::Other(err.to_string()))?;
        }

        self.set_workload_state(InstallerWorkloadState::Done);
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Product {
    pub name: String,
    pub publisher: String,
    pub product_url: String,
    pub control_script: String,
    pub target_directory: String,
    pub repository: String
}

impl Product{
    pub fn get_path_to_package(&self, package: &Package) -> &std::path::Path {
        std::path::Path::new(&self.target_directory)
    }

    pub fn get_uri_to_package(&self, package: &Package) -> String {
        format!("{}packages/{}", self.repository, package.archive)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Repository {
    pub application_name: String,
    pub packages: Vec<Package>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Package {
    pub name: String,
    pub display_name: String,
    pub version: String,
    pub release_date: String,
    pub default: bool,
    pub archive: String,
    pub sha1: String
}

pub struct PackageFile {
    pub handle: tempfile::NamedTempFile,
    pub package: Package
}

pub struct InstallitionSummary {
    file: std::fs::File,
    inner: InstallitionSummaryInner
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct InstallitionSummaryInner {
    pub application_name: String,
    pub packages: Vec<PackageInstallition>,
}

impl Deref for InstallitionSummary {
    type Target = InstallitionSummaryInner;
    
    fn deref(&self) -> &Self::Target {
         &self.inner   
    }
}  

impl DerefMut for InstallitionSummary {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl InstallitionSummary {
    pub fn read_or_create(base: &PathBuf) -> Result<Self, WeakStructParseError> {
        let struct_path = base.join("instally_summary.xml");

        let mut file = match std::fs::File::options()
            .read(true).write(true).open(struct_path.clone()) {
            Ok(f) => f,
            Err(err) =>  {
                log::info!("Failed to open installition summary file. Creating new one. Trace: {}", err);
                std::fs::File::options().create(true).read(true)
                    .write(true).open(struct_path.clone())?
            }
        };

        let mut weak_struct = String::new();
        file.read_to_string(&mut weak_struct)?;

        let inner: InstallitionSummaryInner = match quick_xml::de::from_str(&weak_struct) {
            Ok(r) => r,
            Err(some) => {
                log::info!("Failed to deserialize installition summary file. Using default. Trace: {:?}", some);
                InstallitionSummaryInner { 
                    application_name: "".to_string(),
                    packages: Vec::<PackageInstallition>::default() 
                }
            }
        }; 

        Ok(InstallitionSummary { file, inner })
    }

    pub fn installed(&mut self, package: Package, files: Vec<std::path::PathBuf>) -> &mut Self {
        //TODO: inspect
        if self.packages.iter().any(|n| n.name == package.name){
            return self;
        }

        self.packages.push(PackageInstallition {
            name: package.name,
            display_name: package.display_name,
            version: package.version,
            default: package.default,
            installed_at: chrono::Local::now(),
            updated_at: chrono::Local::now(),
            files: files,
        });

        log::info!("+ Installed packages: {}", self.packages.iter().map(|e| e.display_name.clone()).collect::<Vec<_>>().join(", "));

        self
    }

    pub fn removed(&mut self, name: &str) -> Result<&mut Self, PackageUninstallError> {
        //TODO: inspect    
        for (index, elem) in self.packages.iter().enumerate() {
            if elem.name == name {
                self.packages.remove(index);
                return Ok(self)
            }
        }

        Err(PackageUninstallError::InstallitionNotFound)
    }

    pub fn find(&self, package: &Package) -> Option<PackageInstallition> {
        self.packages.iter().find(|n| n.name == package.name)
            .map(|f| f.clone())
    }

    pub fn packages(&self) -> &[PackageInstallition] {
        &self.packages
    }

    pub fn packages_mut(&mut self) -> &mut [PackageInstallition] {
        &mut self.packages
    } 

    pub fn save(&mut self) -> Result<&mut Self, WeakStructParseError> {
        let xml_decl = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\" ?>\n";
        let xml_str = quick_xml::se::to_string(&self.inner)?;
    
        let mut xml = xml_decl.to_vec();
        xml.extend(xml_str.as_bytes());

        log::trace!("");
        log::trace!("New weak struct state:");
        log::trace!("{:?}", std::str::from_utf8(&xml));
        log::trace!("");

        self.file.set_len(0)?;
        self.file.rewind()?;
        self.file.write_all(&xml)?;

        Ok(self)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct PackageInstallition {
    pub name: String,
    pub display_name: String,
    pub version: String,
    pub installed_at: chrono::DateTime<chrono::Local>,
    pub updated_at: chrono::DateTime<chrono::Local>,
    pub default: bool,
    pub files: Vec<std::path::PathBuf>
}

#[derive(Debug, Clone)]
pub enum InstallerWorkloadState {
    FetchingRemoteTree(String),
    DownloadingComponent(String),
    InstallingComponent(String),
    Interrupted(String),
    Aborted,
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

            InstallerWorkloadState::Interrupted(str) => {
                write!(f, "Interrupted due error: {}", str)
            },
            
            InstallerWorkloadState::Aborted => {
                write!(f, "Aborted by user request")
            },
            
            InstallerWorkloadState::Done => {
                write!(f, "Installition is completed")
            }
        }
    }
}

impl Default for InstallerWorkloadState {
    fn default() -> Self {
        Self::FetchingRemoteTree("".to_string())
    }
}
