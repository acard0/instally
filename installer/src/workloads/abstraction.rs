
use std::{fmt::{Display, self}, sync::Arc, result};
use async_trait::async_trait;
use error_stack::{Result, Context, IntoReport, ResultExt};
use filepath::FilePath;
use parking_lot::{Mutex};
use serde_xml_rs::from_str;

use crate::{http::{client::{self, HttpStreamError}}, archiving};

use super::installer::{Product, Repository, Package, PackageFile};

#[derive(Clone)]
pub enum WorkloadResult {
    Ok,
    Error(String)
}

#[derive(Clone)]
pub struct AppContext<TState>
where TState: Display + Send + Clone + 'static {
    pub frame_count: i32,
    
    state: Option<TState>,
    state_progress: f32,
    result: Option<WorkloadResult>, 
}

#[derive(Clone)]
pub struct InstallyApp<TState>
where TState: Display + Send + Clone + 'static {
    pub product: Product,
    pub context: Arc<Mutex<AppContext<TState>>>,
}

#[derive(Debug, Clone)]
pub struct WorkloadError {}
impl Context for WorkloadError {}
impl fmt::Display for WorkloadError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("Failed to complete the workload gracefully")
    }
}

#[derive(Debug, Clone)]
pub enum RepositoryFetchError {
    NetworkError(String),
    ParseError(String),
}
impl Context for RepositoryFetchError {}
impl fmt::Display for RepositoryFetchError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("RepositoryFetchError: Failed to fetch repository from repo uri")
    }
}

#[derive(Debug, Clone)]
pub enum PackageDownloadError {
    NetworkError(String),
    IOError(String)
}
impl Context for PackageDownloadError {}
impl fmt::Display for PackageDownloadError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("DownloadPackageError: Failed to download the package")
    }
}

#[derive(Debug, Clone)]
pub enum PackageInstallError {
    IOError(String),
    ArchiveError(String)
}
impl Context for PackageInstallError {}
impl fmt::Display for PackageInstallError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("PackageDownloadError: Failed to install the package")
    }
}
impl From<zip::result::ZipError> for PackageInstallError {
    fn from(value: zip::result::ZipError) -> Self {
        match value {
            zip::result::ZipError::Io(r) => PackageInstallError::IOError(r.to_string()),
            zip::result::ZipError::InvalidArchive(r) => PackageInstallError::ArchiveError(r.to_string()),
            zip::result::ZipError::UnsupportedArchive(r) => PackageInstallError::ArchiveError(r.to_string()),
            _ => PackageInstallError::IOError(format!("Failed to find the archive file"))
        }
    }
}

pub trait ContextAccessor<TState>
where TState: Display + Send + Clone + 'static {
    fn get_context(&self) -> Arc<Mutex<AppContext<TState>>>;
    fn get_product(&self) -> Product;
}

#[async_trait]
pub(crate) trait Workload<TState>
where TState: Display + Send + Clone + 'static {
    async fn run(&self) -> Result<(), WorkloadError>;
}

#[async_trait]
pub(crate) trait Worker<TState>: Workload<TState> + ContextAccessor<TState>
where TState: Display + Send + Clone + 'static {
    fn set_workload_state(&self, n_state: TState) {
        self.get_context().lock().state = Some(n_state)
    }

    fn set_state_progress(&self, n_progress: f32) {
        self.get_context().lock().state_progress = n_progress;
    }

    fn set_result(&self, result: WorkloadResult) {
        self.get_context().lock().result = Some(result)
    }

    async fn fetch_repository(&self) -> Result<Repository, RepositoryFetchError>{
        let product = self.get_product();

        let xml_uri = format!("{}meta.xml", &product.repository);
        let xml = self.get_text(&xml_uri).await
            .change_context(RepositoryFetchError::NetworkError(format!("Failed to fetch repository for {}", &product.name)))
            .attach_printable_lazy(|| format!("Failed to fetch repository for {}", &product.name))?;

        let repository: Repository = from_str(&xml)
            .into_report()
            .change_context(RepositoryFetchError::ParseError(format!("Failed to parse repository for {} from xml str", &product.name)))
            .attach_printable_lazy(|| format!("Failed to parse repository for {} from xml str", &product.name))?;

        log::info!("Fetched and parsed Repository structure for {}", product.name);
        Ok(repository)
    }

    async fn get_package(&self, package: &Package) -> Result<PackageFile, PackageDownloadError>{
        let product = self.get_product();

        let mut file = tempfile::NamedTempFile::new()
            .into_report()
            .change_context(PackageDownloadError::IOError(format!("Failed to create temporary file")))
            .attach_printable(format!("Failed to create temporary file"))?;

        let path_buff = file.as_file().path().into_report()
            .change_context(PackageDownloadError::IOError(format!("Failed to accure file path from underlaying std::fs::File strct")))
            .attach_printable(format!("Failed to accure file path from underlaying std::fs::File strct"))?;
        let path = path_buff.to_str().unwrap().to_owned(); // its wt8 buffer. should never cause a problem

        let file_download_result = self.get_file(&product.get_uri_to_package(&package), file.as_file_mut()).await;

        match file_download_result {
            Ok(()) => Ok(PackageFile { handle: file, package: package.clone() }),
            
            Err(err) => {
                Err(err.change_context(PackageDownloadError::NetworkError(format!("Failed to transfer stream chunks into the file {}", path)))
                    .attach_printable(format!("Failed to transfer stream chunks into the file {}", path)))
            }
        }
    }

    async fn install_package(&self, package: &Package, package_file: &PackageFile) -> Result<(), PackageInstallError> {
        let product = self.get_product();
        let progress_closure = self.create_progress_closure();
        archiving::zip_read::extract_to(&package_file.handle.as_file(), product.get_path_to_package(package), &progress_closure)
            .map_err(|e| PackageInstallError::from(e))
            .into_report()
    }

    async fn get_file(&self, url: &str, file: &mut std::fs::File) -> Result<(), HttpStreamError> {
        let progress_closure = self.create_progress_closure();
        client::get_file(url, file, progress_closure).await
    }
    
    async fn get_text(&self, url: &str) -> Result<String, HttpStreamError> {
        let progress_closure = self.create_progress_closure();
        client::get_text(url, progress_closure).await
    }

    fn create_progress_closure(&self) -> Box<dyn Fn(f32) + Send> {
        let arc = self.get_context();
        Box::new(move |progress: f32| {
            arc.lock().state_progress = progress; 
        })
    }
}

impl<TState> InstallyApp<TState>
where TState: Display + Send + Clone + 'static {
    fn get_context(&self) -> Arc<Mutex<AppContext<TState>>> {
        self.context.clone()
    }
}

impl<TState> ContextAccessor<TState> for InstallyApp<TState>
where TState: Display + Send + Clone + 'static {
    fn get_context(&self) -> Arc<Mutex<AppContext<TState>>> {
        self.context.clone()
    }

    fn get_product(&self) -> Product {
        self.product.clone()
    }
}

impl<TState> AppContext<TState>
where TState: Display + Send + Clone + 'static {
    pub fn is_completed(&self) -> bool {
        self.get_result().is_some()
    }

    pub fn is_error(&self) -> bool {
        match self.get_result() {
            Some(WorkloadResult::Error(err)) => true,
            _ => false
        }
    }

    pub fn get_state_information(&self) -> String {
        self.get_state_information_fallback("")
    }

    pub fn get_state_information_fallback(&self, fallback: &str) -> String {
        match self.get_state() {
            None => fallback.to_owned(),
            Some(str) => str.to_string()
        }
    }

    pub fn get_state(&self) -> Option<TState> {
        match &self.state {
            Some(st) => Some(st.clone()),
            _ => None
        }
    }

    pub fn get_result(&self) -> Option<WorkloadResult> {
        match &self.result {
            Some(st) => Some(st.clone()),
            _ => None
        }
    }

    pub fn get_progress(&self) -> f32 {
        self.state_progress
    }  
}

impl<TState> Default for InstallyApp<TState>
where TState: Display + Send + Clone + 'static {
    fn default() -> Self {
        InstallyApp { 
            context: Arc::new(Mutex::new(AppContext::default())),
            product: Product::default()
        }
    }
}

impl<TState> InstallyApp<TState>
where TState: Display + Send + Clone + 'static {
    pub fn new(prdct: Product) -> Self {
        InstallyApp {
            context: Arc::new(Mutex::new(AppContext::default())),
            product: prdct, 
        }
    }
}

impl<TState> Default for AppContext<TState>
where TState: Display + Send + Clone + 'static {
    fn default() -> Self {
        AppContext {
            frame_count: 0,
            state_progress: 0.0,
            state: None,
            result: None,
        }
    }
}

impl WorkloadResult {
    pub fn is_ok(&self) -> bool {
        match self {
            Self::Ok => true,
            _ => false
        }
    }

    pub fn get_error(&self) -> Option<String> {
        match self {
            Self::Error(err) => Some(err.clone()),
            _ => None,
        }
    }
}
