
use std::{fmt::{Display}, sync::Arc, io::Read};
use async_trait::async_trait;

use filepath::FilePath;
use parking_lot::{Mutex};
use serde_xml_rs::from_str;

use crate::{http::{client::{self, HttpStreamError}}, archiving};

use super::{installer::{Product, Repository, Package, PackageFile, InstallitionSummary}, errors::*};

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

pub trait ContextAccessor<TState>
where TState: Display + Send + Clone + 'static {
    fn get_context(&self) -> Arc<Mutex<AppContext<TState>>>;
    fn get_product(&self) -> Product;
}

#[async_trait]
pub(crate) trait Workload<TState> 
where  TState: Display + Send + Clone + 'static {      
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
        let xml = self.get_text(&xml_uri).await?;

        let repository: Repository = from_str(&xml)?;

        log::info!("Fetched and parsed Repository structure for {}", product.name);
        Ok(repository)
    }

    async fn get_package(&self, package: &Package) -> Result<PackageFile, PackageDownloadError>{
        let product = self.get_product();

        let mut file = tempfile::NamedTempFile::new()?;

        let path_buff = file.as_file().path()?;
        let path = path_buff.to_str().unwrap().to_owned(); // its wt8 buffer. should never cause a problem

        let file_download_result = self.get_file(&product.get_uri_to_package(&package), file.as_file_mut()).await?;

        Ok(PackageFile { handle: file, package: package.clone() })
    }

    async fn install_package(&self, package: &Package, package_file: &PackageFile) -> Result<(), PackageInstallError> {
        let product = self.get_product();
        let progress_closure = self.create_progress_closure();
        archiving::zip_read::extract_to(&package_file.handle.as_file(), product.get_path_to_package(package), &progress_closure)?;

        Ok(())
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

    fn get_installition_summary(&self) -> Result<InstallitionSummary, WeakStructParseError> {
        let path = std::env::current_dir().unwrap();
        let struct_path = path.join(".instally.summary.xml");
        let product = self.get_product();

        let mut file = std::fs::File::open(struct_path)?;

        let mut weak_struct = String::new();
        file.read_to_string(&mut weak_struct)?;

        let repository: InstallitionSummary = from_str(&weak_struct)?; 

        Ok(repository)
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
