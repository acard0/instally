
use std::{ops::{Deref, DerefMut}, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};

use crate::helpers::{self, serializer::{self, SerializationError}, versioning::version_compare};

use super::{error::PackageUninstallError, operation::OperationHistory, package::Package, product::Product};

#[derive(Clone, Debug)]
pub struct InstallationSummary {
    path: std::path::PathBuf,
    inner: InstallitionSummaryInner
}

impl Default for InstallationSummary {
    fn default() -> Self {
        Self { path: Default::default(), inner: Default::default() }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct InstallitionSummaryInner {
    pub application_name: String,
    pub packages: Vec<PackageInstallation>,
    pub operations: OperationHistory,
}

impl Deref for InstallationSummary {
    type Target = InstallitionSummaryInner;
    
    fn deref(&self) -> &Self::Target {
         &self.inner   
    }
}  

impl DerefMut for InstallationSummary {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl InstallationSummary {
    /// Creates a new InstallitionSummary for specified 'Product'
    pub(crate) fn default(product: &Product) -> Self {
        let path = std::path::PathBuf::from(&product.target_directory).join("instally_summary.json");
        InstallationSummary {
            path: path.into(),
            inner: InstallitionSummaryInner { 
                application_name: product.name.clone(),
                packages: Vec::<PackageInstallation>::default(),
                operations: OperationHistory::default()
            }
        }
    }

    /// Attempts to read installation summary from cwdir
    pub(crate) fn read() -> Result<Self, SerializationError> {
        let struct_path = Path::new("instally_summary.json");
        let summary: InstallitionSummaryInner = serializer::from_json_file(&struct_path)?;
        Ok(InstallationSummary { path: struct_path.to_path_buf(), inner: summary })
    }

    /// Attempts to read installation summary at installation directory, creating new one if not present
    pub(crate) fn read_or_create_target(product: &Product) -> Result<Self, SerializationError> {
        Self::read_or_create(product, &std::path::PathBuf::from(&product.target_directory))
    }

    /// Attempts to read installation summary at cwdir, creating new one if not present
    pub(crate) fn read_or_create(product: &Product, base: &PathBuf) -> Result<Self, SerializationError> {
        let struct_path = base.join("instally_summary.json");

        let mut file = match helpers::file::open(struct_path.clone()) {
            Ok(f) => f,
            Err(err) =>  {
                log::info!("Failed to open installition summary file. Creating new one. {}", err);
                helpers::file::create_dir_all(base)?; // ensure path is existing
                helpers::file::open_create(struct_path.clone())?
            }
        };

        let mut weak_struct = String::new();
        helpers::file::read_to_string_from_file(&mut file, &mut weak_struct)?;
        weak_struct = product.create_formatter().format(&weak_struct);

        let inner: InstallitionSummaryInner = match serializer::from_json(&weak_struct) {
            Ok(r) => r,
            Err(some) => {
                log::info!("Failed to deserialize installition summary file. Using default. {:?}", some);
                InstallitionSummaryInner { 
                    application_name: product.name.clone(),
                    packages: Vec::<PackageInstallation>::default(),
                    operations: OperationHistory::default()
                }
            }
        }; 

        Ok(InstallationSummary { path: struct_path, inner })
    }
 
    /// Attempts to find package installation metadata from the installation summary
    pub fn find(&self, package: &Package) -> Option<&PackageInstallation> {
        for next in &self.packages {
            if next.name == package.name {
                return Some(next);
            }
        }
        
        None
    }

    pub fn find_mut(&mut self, package: &Package) -> Option<&mut PackageInstallation> {
        for next in &mut self.packages {
            if next.name == package.name {
                return Some(next);
            }
        }
        
        None
    }
    
    /// Checks available updates for specified 'packages'
    pub fn cross_check(&self, packages: &[Package]) -> CrossCheckSummary {
        let mut updates = vec![];
        let mut map = vec![];
        let mut not_installed = vec![];
        for remote in packages.iter() {
            match self.find(remote) {
                Some(local) => {
                    map.push( PackagePair { local: local.clone(), remote: remote.clone() } );
        
                    if version_compare(&remote.version, &local.version) == std::cmp::Ordering::Greater{
                        updates.push( PackagePair { local: local.clone(), remote: remote.clone() } );
                    }
                }
                None => { 
                    not_installed.push(remote.clone());
                }
            }
        }

        CrossCheckSummary { 
            map,
            updates,
            not_installed
        }
    }
    
    /// Gets installation metadata of installed packages
    pub fn get_packages(&self) -> &[PackageInstallation] {
        &self.packages
    }

    pub(super) fn add_package(&mut self, package: &Package, operations: OperationHistory) -> &mut PackageInstallation {
        if self.packages.iter().any(|f| f.name == package.name) {
            panic!("Package '{}' is already installed.", package.name)
        }

        self.packages.push(PackageInstallation::from_package(package));
        self.find_mut(package).unwrap()
    }

    pub(super) fn remove_package(&mut self, name: &str) -> Result<&mut Self, PackageUninstallError> {
        //TODO: inspect    
        for (index, elem) in self.packages.iter().enumerate() {
            if elem.name == name {
                self.packages.remove(index);
                return Ok(self)
            }
        }

        Err(PackageUninstallError::InstallationNotFound)
    }
    
    pub(super) fn save(&mut self) -> Result<&mut Self, SerializationError> {
        let mut file = helpers::file::open_create(self.path.clone())?;
        helpers::file::write_all_file(&mut file, serializer::to_json(&self.inner)?.as_bytes())?;
        Ok(self)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct PackageInstallation {
    pub name: String,
    pub display_name: String,
    pub version: String,
    pub installed_at: chrono::DateTime<chrono::Local>,
    pub updated_at: chrono::DateTime<chrono::Local>,
    pub default: bool,
    pub operations: OperationHistory
}

impl PackageInstallation {
    fn from_package(package: &Package) -> PackageInstallation {
        PackageInstallation {
            name: package.name.clone(),
            display_name: package.display_name.clone(),
            version: package.version.clone(),
            default: package.default,
            installed_at: chrono::Local::now(),
            updated_at: chrono::Local::now(),
            operations: OperationHistory::default(),
        }
    }
}

pub struct PackagePair {
    pub local: PackageInstallation,
    pub remote: Package
}

pub struct CrossCheckSummary {
    pub map: Vec<PackagePair>,
    pub updates: Vec<PackagePair>,
    pub not_installed: Vec<Package>
}