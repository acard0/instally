
use std::{fmt::{Display, Formatter}, ops::{Deref, DerefMut}, io::{Read, Write, Seek}, path::PathBuf};

use crate::{workloads::errors::WorkloadError, http::client, helpers::versioning::version_compare};

use super::{abstraction::{Worker, ContextAccessor, Workload, ContextArcM, AppWrapper}, errors::{WeakStructParseError, PackageUninstallError, RepositoryCrossCheckError, RepositoryFetchError}, updater::{PackagePair, CrossCheckSummary}};

use serde::{Deserialize, Serialize};
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

impl Worker for InstallerWrapper { }

impl ContextAccessor for InstallerWrapper {
    fn get_context(&self) -> ContextArcM {
        self.app.get_context()
    }

    fn get_product(&self) -> Product {
        self.app.product.clone()
    }
}

#[async_trait]
impl Workload for InstallerWrapper {
    async fn run(&self) -> Result<(), WorkloadError> {
        log::info!("Starting to install {}", &self.app.product.name);

        self.set_workload_state(InstallerWorkloadState::FetchingRemoteTree(self.app.product.name.clone()));     
        let repository = self.fetch_repository().await
            .map_err(|err| WorkloadError::Other(err.to_string()))?;

        std::fs::create_dir_all(&self.app.product.target_directory)
            .map_err(|err| WorkloadError::Other(err.to_string()))?;

        // api uses product, resovles it from filesystem
        self.get_product().dump()
            .map_err(|e| WorkloadError::Other(e.to_string()))?;

        let targets = match &self.settings.target_packages {
            None => repository.packages,
            Some(t) => t.to_vec()
        };

        log::info!("Packages in installition queue: {}", targets.iter().map(|e| e.display_name.clone()).collect::<Vec<_>>().join(", "));

        for package in targets {

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
        self.set_state_progress(100.0);
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
    pub fn read() -> Result<Product, WeakStructParseError> {
        let mut file = std::fs::OpenOptions::new()
            .read(true).open("product.xml")?;

        let mut xml = String::new();

        file.read_to_string(&mut xml)?;
        let product: Product = quick_xml::de::from_str(&xml)?;

        Ok(product)
    }

    pub fn get_path_to_package(&self, _package: &Package) -> &std::path::Path {
        std::path::Path::new(&self.target_directory)
    }

    pub fn get_uri_to_package(&self, package: &Package) -> String {
        format!("{}packages/{}", self.repository, package.archive)
    }

    pub fn get_path_to_self_struct_target(&self) -> std::path::PathBuf {
        std::path::Path::new(&self.target_directory).join("product.xml")
    }

    pub fn get_path_to_self_struct_local(&self) -> std::path::PathBuf {
        std::env::current_dir().unwrap().join("product.xml")
    }

    pub async fn fetch_repository(&self) -> Result<Repository, RepositoryFetchError> {
        let xml_uri = format!("{}meta.xml", &self.repository);
        let xml = client::get_text(&xml_uri, |_| ()).await?;

        let repository: Repository = quick_xml::de::from_str(&xml)?;

        log::info!("Fetched and parsed Repository structure for {}", self.name);
        Ok(repository)
    }

    pub fn dump(&self) -> Result<(), WeakStructParseError> {
        let payload = quick_xml::se::to_string(&self)?;

        let mut file = std::fs::OpenOptions::new().create(true).write(true).
            read(true).truncate(true).open(&self.get_path_to_self_struct_target())?;

        let xml_decl = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\" ?>\n";
        let mut xml = xml_decl.to_vec();
        xml.extend(payload.as_bytes());

        file.write_all(&xml)?;

        Ok(())
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
    path: std::path::PathBuf,
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
    pub fn read_or_create_target(product: &Product) -> Result<Self, WeakStructParseError> {
        Self::read_or_create(&std::path::PathBuf::from(&product.target_directory))
    }

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

        Ok(InstallitionSummary { path: struct_path, inner })
    }

    
    pub fn find(&self, package: &Package) -> Option<PackageInstallition> {
        self.packages.iter().find(|n| n.name == package.name)
        .map(|f| f.clone())
    }
    
    pub fn cross_check(&self, packages: &[Package]) -> Result<CrossCheckSummary, RepositoryCrossCheckError> {
        let mut updates = vec![];
        let mut map = vec![];

        for remote in packages.iter() {
            match self.find(remote) {
                Some(local) => {
                    map.push( PackagePair { local: local.clone(), remote: remote.clone() } );
        
                    if version_compare(&remote.version, &local.version) == std::cmp::Ordering::Greater{
                        updates.push( PackagePair { local: local.clone(), remote: remote.clone() } );
                    }
                }
                None => { 
                    // package is not installed on local
                }
            }
        }

        Ok(CrossCheckSummary { 
            map,
            updates
        })
    }
    
    pub fn packages(&self) -> &[PackageInstallition] {
        &self.packages
    }
    
    pub fn packages_mut(&mut self) -> &mut [PackageInstallition] {
        &mut self.packages
    } 

    pub fn installed(&mut self, package: Package, files: Vec<std::path::PathBuf>) -> &mut Self {
        let current = self.packages.iter().position(|n| n.name == package.name)
            .and_then(|f| self.packages.get_mut(f));
    
        let remote = PackageInstallition::from_package(&package, files);
    
        match current {
            Some(current) if current.version < remote.version => {
                current.updated_at = chrono::Local::now();
                current.version = remote.version;
                log::info!("~> Updated package: {}", package.name);
                return self;  
            }
            Some(current) if current.version > remote.version => { 
                current.updated_at = chrono::Local::now();
                current.version = remote.version;
                log::info!("<~ Downgraded package: {}", package.name);
            }
            Some(current) if current.version == remote.version => { 
                log::info!("= Reinstalled package: {}", package.name);
            }
            _ => {
                self.packages.push(remote);
                log::info!("+ Installed package: {}", package.name); 
            }
        }
    

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
    
    pub fn save(&mut self) -> Result<&mut Self, WeakStructParseError> {
        let xml_decl = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\" ?>\n";
        let xml_str = quick_xml::se::to_string(&self.inner)?;
    
        let mut xml = xml_decl.to_vec();
        xml.extend(xml_str.as_bytes());

        let mut file = std::fs::File::options().create(true).read(true)
            .write(true).open(self.path.clone())?;

        file.set_len(0)?;
        file.rewind()?;
        file.write_all(&xml)?;

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

impl PackageInstallition {
    fn from_package(package: &Package, files: Vec<PathBuf>) -> PackageInstallition {
        PackageInstallition {
            name: package.name.clone(),
            display_name: package.display_name.clone(),
            version: package.version.clone(),
            default: package.default,
            installed_at: chrono::Local::now(),
            updated_at: chrono::Local::now(),
            files,
        }
    }
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
