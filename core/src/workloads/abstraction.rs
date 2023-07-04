use std::{sync::Arc, collections::HashMap, fmt::Display};

use async_trait::async_trait;
use once_cell::sync::Lazy;
use parking_lot::Mutex;

use crate::{http::{client::{self, HttpStreamError}}, archiving};
use super::{installer::{Product, Repository, Package, PackageFile, InstallitionSummary}, errors::*};

pub type ArcM<T> = Arc<Mutex<T>>;
pub type LazyArcM<T> = Lazy<ArcM<T>>;

pub type ContextArcM = ArcM<AppContext>;

static CONTEXT_CALLBACKS: LazyArcM<HashMap<usize, StateCallbackBox>> = LazyArcM::new(|| ArcM::new(Mutex::new(HashMap::new())));

#[derive(Clone, Debug)]
pub struct AppWrapper<T: Default + Clone> {
    pub app: InstallyApp,
    pub settings: T, 
}

impl<T: Default + Clone> AppWrapper<T> {
    pub fn new(app: InstallyApp) -> Self {
        AppWrapper { 
            app,
            settings: T::default()
        }
    }

    pub fn new_with_opts(app: InstallyApp, settings: T) -> Self {
        AppWrapper { app, settings}
    }
}


#[derive(Clone, Debug)]
pub enum WorkloadResult {
    Ok,
    Error(String)
}

#[derive(struct_field::StructField, Clone, Debug)]
pub struct AppContext {
    frame_count: u64, 
    state: Option<String>,
    state_progress: f32,
    result: Option<WorkloadResult>,
}

impl AppContextNotifiable for AppContext {
    fn on_update(&self, field: AppContextField) {
        CONTEXT_CALLBACKS.lock().iter().for_each(|f| {
            let (_, callback) = f;
            callback(AppContextChange { current_cloned: self.clone(), change_cloned:  field.clone()})
        })
    }

    fn subscribe(&self, action: StateCallbackBox) -> usize {
        let mut map =  CONTEXT_CALLBACKS.lock();
        let id = map.len();
        map.insert(id, action);
        id
    }

    fn unsubscribe(&self, id: usize) -> bool {
        CONTEXT_CALLBACKS.lock().remove(&id).is_some()
    }
}

#[derive(Clone, Debug)]
pub struct InstallyApp {
    pub product: Product,
    pub context: Arc<Mutex<AppContext>>,
}

pub trait ContextAccessor {
    fn get_context(&self) -> Arc<Mutex<AppContext>>;
    fn get_product(&self) -> Product;
}

#[async_trait]
pub trait Workload {      
    async fn run(&self) -> Result<(), WorkloadError>;           
}

#[async_trait]
pub trait Worker: Workload + ContextAccessor {
    fn set_workload_state<S: Display>(&self, n_state: S) {
        let binding = self.get_context();
        let mut ctx = binding.lock(); 
        ctx.update_field(AppContextField::state(Some(n_state.to_string())))
    }

    fn set_state_progress(&self, n_progress: f32) {
        let binding = self.get_context();
        let mut ctx = binding.lock();
        ctx.update_field(AppContextField::state_progress(n_progress));
    }

    fn set_result(&self, result: WorkloadResult) {
        let binding = self.get_context();
        let mut ctx = binding.lock();
        ctx.update_field(AppContextField::result(Some(result)))
    }

    async fn fetch_repository(&self) -> Result<Repository, RepositoryFetchError>{
        self.get_product().fetch_repository().await
    }

    async fn get_package(&self, package: &Package) -> Result<PackageFile, PackageDownloadError>{
        let product = self.get_product();
        let mut file = tempfile::NamedTempFile::new()?;
        let _ = self.get_file(&product.get_uri_to_package(&package), file.as_file_mut()).await?;
        Ok(PackageFile { handle: file, package: package.clone() })
    }

    async fn install_package(&self, package: &Package, package_file: &PackageFile) -> Result<(), PackageInstallError> {
        let product = self.get_product();
        let progress_closure = self.create_progress_closure();
        let files = archiving::zip_read::extract_to(&package_file.handle.as_file(), product.get_path_to_package(package), &progress_closure)?;
        
        self.get_installition_summary_target()?
            .installed(package.clone(), files)
            .save()?;

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

    fn get_installition_summary_local(&self) -> Result<InstallitionSummary, WeakStructParseError> {
        let current = std::env::current_dir().unwrap();
        InstallitionSummary::read_or_create(&current)
    }

    fn get_installition_summary_target(&self) -> Result<InstallitionSummary, WeakStructParseError> {
        InstallitionSummary::read_or_create_target(&self.get_product())
    }
}

impl ContextAccessor for InstallyApp
{
    fn get_context(&self) -> Arc<Mutex<AppContext>> {
        self.context.clone()
    }

    fn get_product(&self) -> Product {
        self.product.clone()
    }
}

impl AppContext
{
    pub fn is_completed(&self) -> bool {
        self.get_result().is_some()
    }

    pub fn is_error(&self) -> bool {
        match self.get_result() {
            Some(WorkloadResult::Error(_)) => true,
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

    pub fn get_state(&self) -> Option<String> {
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

impl Default for InstallyApp
{
    fn default() -> Self {
        InstallyApp { 
            context: Arc::new(Mutex::new(AppContext::default())),
            product: Product::default()
        }
    }
}

impl InstallyApp
{
    pub fn new(prdct: Product) -> Self {
        InstallyApp {
            context: Arc::new(Mutex::new(AppContext::default())),
            product: prdct, 
        }
    }
}

impl Default for AppContext
{
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
