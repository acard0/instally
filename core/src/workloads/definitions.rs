use std::{collections::HashMap, ffi::{c_char, CStr}, io::{Read, Write}, ops::{Deref, DerefMut}, path::{Path, PathBuf}, process::Command, sync::Arc};

use crate::{helpers::{self, formatter::TemplateFormat, serializer::{self, SerializationError}, versioning::version_compare, workflow::{self, Workflow}}, http::client, scripting::{builder::{IJSContext, IJSRuntime}, error::IJSError}};

use super::{abstraction::InstallyApp, error::{PackageUninstallError, RepositoryFetchError, ScriptError}};

use directories::UserDirs;
use rust_i18n::Backend;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Product {
    pub name: String,
    pub title: String,
    pub publisher: String,
    pub product_url: String,
    pub repository: String,
    pub script: String,
    target_directory: String,
}

impl Product{
    pub fn new(name: &str, title: &str, publisher: &str, product_url: &str, repository: &str, script: &str, target_directory: &str) -> Self {
        Product {
            name: name.to_owned(),
            title: title.to_owned(),
            publisher: publisher.to_owned(),
            product_url: product_url.to_owned(),
            repository: repository.to_owned(),
            script: script.to_owned(),
            target_directory: target_directory.to_owned()
        }
    }

    pub fn read_template<P: AsRef<Path>>(path: P) -> Result<Product, SerializationError> {
        let template: Product = serializer::from_file(path)?;
        Ok(template)
    }

    pub fn read() -> Result<Product, SerializationError> {
        Self::read_file(Path::new("product.xml"))
    }

    pub fn read_file<P: AsRef<Path>>(path: P) -> Result<Product, SerializationError> {
        let template: Product = serializer::from_file(path)?;
        Self::from_template(template)
    }

    pub fn from_template(template: Product) -> Result<Product, SerializationError> {
        let formatter = template.create_formatter();
        let back_step = serializer::to_xml(&template).unwrap();
        let xml = formatter.format(&back_step);

        let product: Product = serializer::from_str(&xml)?;
        let current = std::env::current_dir().unwrap();
        let target = &product.target_directory;

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

    pub fn get_path_to_package(&self, _package: &Package) -> std::path::PathBuf {
        self.get_relative_target_directory()
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

    pub fn get_uri_to_global_script(&self, repository: &Repository) -> Option<String> {
        // product struct also contains script field but if for some unknown reason
        // script file name at cloud gets changed it can cause issue as product struct is embeded.
        // also product struct has to contain script name field because it will be used at binary generation

        if repository.script.is_empty() {
            return None
        }
        
        Some(format!("{}{}", self.repository, repository.script))
    }

    pub fn get_path_to_self_struct_target(&self) -> std::path::PathBuf {
        std::path::Path::new(&self.target_directory).join("product.xml")
    }

    pub fn get_path_to_self_struct_local(&self) -> std::path::PathBuf {
        std::env::current_dir().unwrap().join("product.xml")
    }

    pub fn get_relative_target_directory(&self) -> std::path::PathBuf {
        match workflow::get_workflow() {
            Workflow::FreshInstallition => {
                std::path::Path::new(&self.target_directory).to_path_buf()
            }
            _ => {
                std::env::current_dir().unwrap()
            }
        }  
    }

    pub fn get_target_directory(&self) -> std::path::PathBuf {
        std::path::Path::new(&self.target_directory).to_path_buf()
    }

    pub async fn fetch_repository(&self) -> Result<Repository, RepositoryFetchError> {
        let xml_uri = format!("{}repository.xml", &self.repository);
        let mut xml = client::get_text(&xml_uri, |_| ()).await?;
        xml = self.create_formatter().format(&xml);

        let repository: Repository = serializer::from_str(&xml)?;

        log::info!("Fetched and parsed Repository structure for {}", self.name);
        Ok(repository)
    }

    pub fn dump(&self) -> Result<(), SerializationError> {
        let mut file = helpers::file::open_create(&self.get_path_to_self_struct_target())?;

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
    pub size: u64
}

impl Repository {
    pub fn new(application_name: &str, size: u64) -> Self {
        Self {
            application_name: application_name.to_string(),
            script: String::new(),
            packages: Vec::new(),
            size
        }
    }

    pub fn get_package(&self, package_name: &str) -> Option<Package> {
        self.packages.iter().find(|e| e.name == package_name)
            .map(|f| f.clone())
    }

    pub fn get_default_packages(&self) -> Vec<Package> {
        self.packages.iter()
            .filter(|e| e.default)
            .cloned()
            .collect()
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
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<PackageDefinition, SerializationError> {
        Ok(serializer::from_file(path)?)
    }

    pub fn define(&self, archive: &str, size: u64, sha1: &str, script: &str) -> Package {
        Package::from_definition(self, archive, size, sha1, script)
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
    pub size: u64,
    pub sha1: String,
    pub script: String
}

impl Package {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Package, SerializationError> {
        Ok(serializer::from_file(path)?)
    }

    pub fn from_definition(definition: &PackageDefinition, archive: &str, size: u64, sha1: &str, script: &str) -> Package {
        Package {
            name: definition.name.clone(),
            display_name: definition.display_name.clone(),
            version: definition.version.clone(),
            release_date: definition.release_date.clone(),
            default: definition.default,
            archive: archive.to_owned(),
            sha1: sha1.to_owned(),
            script: script.to_owned(),
            size
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
        let ctx = rt.create_context(&app);
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
    pub fn execute(self, arguments: Vec<String>, attached: bool) -> std::io::Result<()> {
        let (_, path) = self.handle.keep().unwrap();

        let mut cmd = Command::new(format!("{}", path.to_str().unwrap()));
        cmd.args(arguments);

        let handle = match cmd.spawn() {
            Ok(handle) => handle,
            Err(err) => {
                log::trace!("Command {:?} failed with error {:?}", cmd, err);
                return Err(err)
            }
        };

        if attached {
            match handle.wait_with_output() {
                Ok(output) => {
                    if !output.status.success() {
                        log::trace!("Command {:?} failed with error {:?}", cmd, output);
                    }
                }
                Err(err) => {
                    log::trace!("Command {:?} failed with error {:?}", cmd, err);
                    return Err(err)
                }
            }
        }

        std::fs::remove_file(path)
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
    pub fn read() -> Result<Self, SerializationError> {
        let struct_path = Path::new("instally_summary.xml");
        let summary: InstallitionSummaryInner = serializer::from_file(struct_path)?;
        Ok(InstallitionSummary { path: struct_path.to_path_buf(), inner: summary })
    }

    pub fn read_or_create_target(product: &Product) -> Result<Self, SerializationError> {
        Self::read_or_create(product, &std::path::PathBuf::from(&product.target_directory))
    }

    pub fn read_or_create(product: &Product, base: &PathBuf) -> Result<Self, SerializationError> {
        let struct_path = base.join("instally_summary.xml");

        let mut file = match helpers::file::open(struct_path.clone()) {
            Ok(f) => f,
            Err(err) =>  {
                log::info!("Failed to open installition summary file. Creating new one. Trace: {}", err);
                helpers::file::open_create(struct_path.clone())?
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
    
    pub fn cross_check(&self, packages: &[Package]) -> Result<CrossCheckSummary, RepositoryFetchError> {
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
    
    pub fn save(&mut self) -> Result<&mut Self, SerializationError> {
        let mut file = helpers::file::open_create(self.path.clone())?;
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

#[derive(Debug, Clone)]
pub struct I18n {
    inner: I18nHolder,
}

impl Deref for I18n {
    type Target = I18nHolder;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl I18n {
    pub fn new() -> Self {
        Self { inner: I18nHolder::new() }
    }

    pub fn get(&self, key: &str) -> String {
        self.translate(&rust_i18n::locale(), key)
            .unwrap_or(key.to_owned())
    }
}

impl rust_i18n::Backend for I18n {
    fn available_locales(&self) -> Vec<String> {
        self.trs.lock().keys().cloned().collect()
    }

    fn translate(&self, locale: &str, key: &str) -> Option<String> {
        self.trs.lock().get(locale)?.get(key).cloned()
    }

    fn add(&mut self, locale: &str, key: &str, value: &str) {
        let mut trs = self.trs.lock();
        let locale = trs.entry(locale.to_string())
            .or_insert_with(HashMap::new);

        locale.insert(key.to_string(), value.to_string());
    }
}

#[derive(Debug, Clone)]
pub struct I18nHolder {
    trs: Arc<parking_lot::Mutex<HashMap<String, HashMap<String, String>>>>,
}

impl I18nHolder {
    pub fn new() -> Self {
        Self { 
            trs: Arc::new(parking_lot::Mutex::new(HashMap::new()))
        }
    }
}
    
#[repr(C)]
pub struct ByteBuffer {
    ptr: *mut u8,
    length: i32,
    capacity: i32,
}

impl ByteBuffer {
    pub fn ptr(&self) -> *mut u8 {
        self.ptr
            .try_into()
            .expect("invalid pointer")
    }

    pub fn cap(&self) -> usize {
        self.capacity
            .try_into()
            .expect("buffer cap negative or overflowed")
    }

    pub fn len(&self) -> usize {
        self.length
            .try_into()
            .expect("buffer length negative or overflowed")
    }

    pub fn from_vec(bytes: Vec<u8>) -> Self {
        let length = i32::try_from(bytes.len()).expect("buffer length cannot fit into a i32.");
        let capacity =
            i32::try_from(bytes.capacity()).expect("buffer capacity cannot fit into a i32.");

        let mut v = std::mem::ManuallyDrop::new(bytes);

        Self {
            ptr: v.as_mut_ptr(),
            length,
            capacity,
        }
    }

    pub fn from_vec_struct<T: Sized>(bytes: Vec<T>) -> Self {
        let element_size = std::mem::size_of::<T>() as i32;

        let length = (bytes.len() as i32) * element_size;
        let capacity = (bytes.capacity() as i32) * element_size;

        let mut v = std::mem::ManuallyDrop::new(bytes);

        Self {
            ptr: v.as_mut_ptr() as *mut u8,
            length,
            capacity,
        }
    }

    pub fn into_slice<T: Sized>(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr() as *mut T, self.len() / std::mem::size_of::<T>()) }
    }

    pub fn into_string_vec(&self) -> Vec<String> {
        self.into_slice::<*mut c_char>()
        .iter().map(|f| unsafe { CStr::from_ptr(*f).to_str().unwrap().to_string() })
        .collect::<Vec<_>>()
    }
    
    pub fn destroy_into_vec(self) -> Vec<u8> {
        if self.ptr.is_null() {
            vec![]
        } else {
            let capacity: usize = self
                .capacity
                .try_into()
                .expect("buffer capacity negative or overflowed");
            let length: usize = self
                .length
                .try_into()
                .expect("buffer length negative or overflowed");

            unsafe { Vec::from_raw_parts(self.ptr, length, capacity) }
        }
    }

    pub fn destroy_into_vec_struct<T: Sized>(self) -> Vec<T> {
        if self.ptr.is_null() {
            vec![]
        } else {
            let element_size = std::mem::size_of::<T>() as i32;
            let length = (self.length * element_size) as usize;
            let capacity = (self.capacity * element_size) as usize;

            unsafe { Vec::from_raw_parts(self.ptr as *mut T, length, capacity) }
        }
    }

    pub fn destroy(self) {
        drop(self.destroy_into_vec());
    }
}