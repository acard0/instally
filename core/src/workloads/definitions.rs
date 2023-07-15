use std::{ops::{Deref, DerefMut}, io::{Read, Write, Seek}, path::{PathBuf, Path}, process::Command};

use crate::{http::client, helpers::{versioning::version_compare, formatter::TemplateFormat, serializer}, scripting::{builder::{IJSContext, IJSRuntime}, error::IJSError}};

use super::{abstraction::InstallyApp, error::{WeakStructParseError, PackageUninstallError, RepositoryCrossCheckError, RepositoryFetchError, ScriptError}};

use directories::UserDirs;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Product {
    pub name: String,
    pub publisher: String,
    pub product_url: String,
    pub target_directory: String,
    pub repository: String,
    pub script: String,
}

impl Product{
    pub fn read() -> Result<Product, WeakStructParseError> {
        Self::read_file(Path::new("product.xml"))
    }

    pub fn read_file<P: AsRef<Path>>(path: P) -> Result<Product, WeakStructParseError> {
        let product: Product = serializer::from_file(path)?;
        let formatter = product.create_formatter();
        let back_step = serializer::to_xml(&product)?;
        let xml = formatter.format(&back_step);
        let product: Product = serializer::from_str(&xml)?;
        Ok(product)
    }

    pub fn create_formatter(&self) -> TemplateFormat {
        let directories = UserDirs::new().unwrap(); 

        TemplateFormat::new()
            .add_replacement("System.Os.Name", std::env::consts::OS)
            .add_replacement("System.Os.Version", std::env::var_os("VERSION").unwrap_or("N/A".into()).to_str().unwrap())
            .add_replacement("App.Name", &self.name)
            .add_replacement("App.Publisher", &self.publisher)
            .add_replacement("App.ProductUrl", &self.product_url)
            .add_replacement("App.TargetDirectory", &self.target_directory)
            .add_replacement("App.Repository", &self.repository)
            .add_replacement("Directories.User.Home", directories.home_dir().to_str().unwrap())
            .add_replacement("Directories.User.Documents", directories.document_dir().unwrap().to_str().unwrap())
            .add_replacement("Directories.User.Downloads", directories.download_dir().unwrap().to_str().unwrap())
            .add_replacement("Directories.User.Desktop", directories.desktop_dir().unwrap().to_str().unwrap())
    }

    pub fn get_path_to_package(&self, _package: &Package) -> &std::path::Path {
        std::path::Path::new(&self.target_directory)
    }

    pub fn get_uri_to_package(&self, package: &Package) -> String {
        format!("{}packages/{}", self.repository, package.archive)
    }

    pub fn get_uri_to_package_script(&self, package: &Package) -> Result<Option<String>, ScriptError> {
        if package.script.is_empty() {
            return Ok(None)
        }
        
        Ok(Some(format!("{}packages/{}", self.repository, package.script)))
    }

    pub async fn get_uri_to_global_script(&self) -> Result<Option<String>, ScriptError> {
        // product struct also contains script field but if for some unknown reason
        // script file name at cloud gets changed it can cause issue as product struct is embeeded
        // product field has to contain script name field because it will be used at binary generation
        let repo = self.fetch_repository().await
            .map_err(|err| ScriptError::Other(format!("Attemptted to get script global script meta but {err:?}")))?;

        if repo.script.is_empty() {
            return Ok(None)
        }
        
        Ok(Some(format!("{}{}", self.repository, repo.script)))
    }

    pub fn get_path_to_self_struct_target(&self) -> std::path::PathBuf {
        std::path::Path::new(&self.target_directory).join("product.xml")
    }

    pub fn get_path_to_self_struct_local(&self) -> std::path::PathBuf {
        std::env::current_dir().unwrap().join("product.xml")
    }

    pub async fn fetch_repository(&self) -> Result<Repository, RepositoryFetchError> {
        let xml_uri = format!("{}repository.xml", &self.repository);
        let mut xml = client::get_text(&xml_uri, |_| ()).await?;
        xml = self.create_formatter().format(&xml);

        let repository: Repository = serializer::from_str(&xml)?;

        log::info!("Fetched and parsed Repository structure for {}", self.name);
        Ok(repository)
    }

    pub fn dump(&self) -> Result<(), WeakStructParseError> {
        let mut file = std::fs::OpenOptions::new().create(true).write(true).
            read(true).truncate(true).open(&self.get_path_to_self_struct_target())?;

        file.write_all(serializer::to_xml(self)?.as_bytes())?;

        Ok(())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Repository {
    pub application_name: String,
    pub script: String,
    pub packages: Vec<Package>,
}

impl Repository {
    pub fn new(application_name: &str) -> Self {
        Self {
            application_name: application_name.to_string(),
            script: String::new(),
            packages: Vec::new(),
        }
    }

    pub fn get_package(&self, package_name: &str) -> Option<Package> {
        self.packages.iter().find(|e| e.name == package_name)
            .map(|f| f.clone())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct PackageDefinition {
    pub name: String,
    pub display_name: String,
    pub version: String,
    pub release_date: String,
    pub default: bool,
    pub script: String
}

impl PackageDefinition {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<PackageDefinition, WeakStructParseError> {
        Ok(serializer::from_file(path)?)
    }

    pub fn define(&self, archive: &str, sha1: &str, script: &str) -> Package {
        Package::from_definition(self, archive, sha1, script)
    }
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
    pub sha1: String,
    pub script: String
}

impl Package {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Package, WeakStructParseError> {
        Ok(serializer::from_file(path)?)
    }

    pub fn from_definition(definition: &PackageDefinition, archive: &str, sha1: &str, script: &str) -> Package {
        Package {
            name: definition.name.clone(),
            display_name: definition.display_name.clone(),
            version: definition.version.clone(),
            release_date: definition.release_date.clone(),
            default: definition.default,
            archive: archive.to_owned(),
            sha1: sha1.to_owned(),
            script: script.to_owned()
        }
    }
}

pub struct PackageFile {
    pub handle: tempfile::NamedTempFile,
    pub package: Package
}

pub struct DependencyFile {
    pub handle: tempfile::NamedTempFile,
}

#[derive(Clone)]
pub struct Script {
    ctx: IJSContext,
}

impl Script {
    pub fn new(src: String, app: &InstallyApp) -> Result<Script, IJSError> {
        let rt = IJSRuntime::current_or_get();
        let ctx = rt.create_context(app.clone());
        ctx.mount(&src)?;
        Ok(Script { ctx })
    }

    pub fn invoke_before_installition(&self) { 
        self.ctx.eval_raw::<()>("Installer.on_before_installition();").unwrap();
    }

    pub fn invoke_after_installition(&self) { 
        self.ctx.eval_raw::<()>("Installer.on_after_installition();").unwrap();
    }

    pub fn invoke_before_update(&self) {
        self.ctx.eval_raw::<()>("Installer.on_before_update();").unwrap();
    }

    pub fn invoke_after_update(&self) { 
        self.ctx.eval_raw::<()>("Installer.on_after_update();").unwrap();
    }

    pub fn invoke_before_uninstallition(&self) {
        self.ctx.eval_raw::<()>("Installer.on_before_uninstallition();").unwrap();
    }

    pub fn invoke_after_uninstallition(&self) {
        self.ctx.eval_raw::<()>("Installer.on_after_uninstallition();").unwrap();
    }
    
    pub fn free(&self) {
        self.ctx.free();
    }
}

pub trait PackageScriptOptional {
    fn if_exist<F: FnOnce(&Script) -> Result<(), ScriptError>>(&self, action: F) -> Result<(), ScriptError>;
}

impl PackageScriptOptional for Option<Script> {
    fn if_exist<F: FnOnce(&Script) -> Result<(), ScriptError>>(&self, action: F) -> Result<(), ScriptError> {
        if let Some(script) = self {
            return action(script);
        }

        Ok(())
    }
}

impl DependencyFile {
    pub fn execute(self, arguments: Vec<String>, attached: bool) {
        let (_, path) = self.handle.keep().unwrap();

        let mut cmd = Command::new(format!("{}", path.to_str().unwrap()));
        arguments.iter().for_each(|f| {
            cmd.arg(f);
        });

        let handle = cmd.spawn().unwrap();

        if attached {
            handle.wait_with_output().unwrap();
        }

        std::fs::remove_file(path).unwrap();
    }
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
        Self::read_or_create(product, &std::path::PathBuf::from(&product.target_directory))
    }

    pub fn read_or_create(product: &Product, base: &PathBuf) -> Result<Self, WeakStructParseError> {
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
        weak_struct = product.create_formatter().format(&weak_struct);

        let inner: InstallitionSummaryInner = match serializer::from_str(&weak_struct) {
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

        Ok(CrossCheckSummary { 
            map,
            updates,
            not_installed
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
        let mut file = std::fs::File::options().create(true).read(true)
            .truncate(true).write(true).open(self.path.clone())?;

        file.write_all(serializer::to_xml(&self.inner)?.as_bytes())?;

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

pub struct PackagePair {
    pub local: PackageInstallition,
    pub remote: Package
}

pub struct CrossCheckSummary {
    pub map: Vec<PackagePair>,
    pub updates: Vec<PackagePair>,
    pub not_installed: Vec<Package>
}