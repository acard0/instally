use std::{collections::HashMap, fmt::{Display, Formatter}, path::Path, sync::Arc};

use async_trait::async_trait;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rust_i18n::error::{Error, ErrorDetails};

use crate::{archiving, helpers::{serializer::SerializationError, tmp, workflow::{self, Workflow}}, http::client::{self, HttpStreamError}, target::error::{AppEntryError, SymlinkError}};

use super::{definitions::*, error::*};

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


#[derive(Debug, Clone)]
pub enum WorkloadResult {
    Ok,
    Error(ErrorDetails)
}

impl Display for WorkloadResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error(err) => write!(f, "{err:?}"),
            _ => write!(f, "Ok")
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

    pub fn get_error(&self) -> Option<ErrorDetails> {
        match self {
            Self::Error(err) => Some(err.clone()),
            _ => None,
        }
    }
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
            callback(AppContextChange { state_cloned: self.clone(), field_cloned:  field.clone()})
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

#[derive(Clone, Debug)]
pub struct InstallyApp {
    product: Product,
    repository: Repository,
    context: Arc<Mutex<AppContext>>,
}

impl InstallyApp
{
    pub async fn build(prdct: &Product) -> Result<Self, RepositoryFetchError> {
        Ok(InstallyApp {
            context: Arc::new(Mutex::new(AppContext::default())),
            product: prdct.clone(), 
            repository: match prdct.fetch_repository().await {
                Ok(it) => it,
                Err(err) => return Err(err),
            },
        })
    }

    pub fn get_context(&self) -> Arc<Mutex<AppContext>> {
        self.context.clone()
    }

    pub fn get_product(&self) -> &Product {
        &self.product
    }

    pub fn get_repository(&self) -> &Repository {
        &self.repository
    }

    pub fn set_workload_state<S: Display>(&self, n_state: S) {
        let mut ctx = self.context.lock(); 
        ctx.update_field(AppContextField::state(Some(n_state.to_string())))
    }

    pub fn set_state_progress(&self, n_progress: f32) {
        let mut ctx = self.context.lock(); 
        ctx.update_field(AppContextField::state_progress(n_progress));
    }

    pub fn set_result(&self, result: WorkloadResult) {
        let mut ctx = self.context.lock(); 
        ctx.update_field(AppContextField::result(Some(result)))
    }

    pub async fn fetch_repository(&self) -> Result<Repository, RepositoryFetchError>{
        self.product.fetch_repository().await
    }

    pub async fn get_package(&self, package: &Package) -> Result<PackageFile, PackageDownloadError>{
        let product = &self.product;

        let mut file = tmp::create_tmp_file()?;
        let _ = self.get_file(&product.get_uri_to_package(&package), file.as_file_mut()).await?;
        Ok(PackageFile { handle: file, package: package.clone() })
    }

    pub async fn get_package_script(&self, package: &Package) -> Result<Option<Script>, ScriptError> {
        self.get_script(self.product.get_uri_to_package_script(package)?).await
    }

    pub async fn get_global_script(&self) -> Result<Option<Script>, ScriptError> {
        self.get_script(self.product.get_uri_to_global_script(self.get_repository())).await
    }

    pub async fn get_script(&self, uri: Option<String>) -> Result<Option<Script>, ScriptError> {
        match uri {
            None => Ok(None),
            Some(uri) => {
                let src = self.get_text(&uri).await?;
                Ok(Some(Script::new(src, self)?))
            }
        } 
    }

    pub async fn get_dependency(&self, uri: &str, state_text: &str) -> Result<DependencyFile, PackageDownloadError>{
        self.set_workload_state(state_text);

        let mut file = tmp::create_tmp_file()?;
        let _ = self.get_file(uri, file.as_file_mut()).await?;
        Ok(DependencyFile { handle: file })
    }

    pub fn install_package(&self, package: &Package, package_file: &PackageFile) -> Result<(), PackageInstallError> {
        let product = &self.product;
        let progress_closure = self.create_progress_closure();
        let files = archiving::zip_read::extract_to(&package_file.handle.as_file(), &product.get_path_to_package(package), &progress_closure)?;
        
        self.get_installition_summary()?
            .installed(package.clone(), files)
            .save()?;

        Ok(())
    }

    pub async fn get_file(&self, url: &str, file: &mut std::fs::File) -> Result<(), HttpStreamError> {
        let progress_closure = self.create_progress_closure();
        client::get_file(url, file, progress_closure).await
    }
    
    pub async fn get_text(&self, url: &str) -> Result<String, HttpStreamError> {
        let progress_closure = self.create_progress_closure();
        client::get_text(url, progress_closure).await
    }

    pub fn get_installition_summary(&self) -> Result<InstallitionSummary, SerializationError> {
        match workflow::get_workflow() {
            Workflow::FreshInstallition => {
                InstallitionSummary::read_or_create_target(&self.product)
            }
            _ => {
                InstallitionSummary::read()
            }
        }
    }

    pub fn symlink_file<P: AsRef<Path>, Q: AsRef<Path>>(&self, original: P, link_dir: Q, link_name: &str) -> Result<(), SymlinkError> {
        crate::sys::symlink_file(original, link_dir, link_name)
    }

    pub fn create_app_entry(&self, app: &InstallyApp, maintenance_tool_name: &str) -> Result<(), AppEntryError> {
        crate::sys::create_app_entry(app, maintenance_tool_name)
    }

    pub fn create_maintenance_tool(&self, app: &InstallyApp, maintenance_tool_name: &str) -> std::io::Result<()> {
        crate::sys::create_maintenance_tool(app, maintenance_tool_name)
    }

    pub fn create_progress_closure(&self) -> Box<dyn Fn(f32) + Send> {
        let arc = self.get_context(); 
        Box::new(move |progress: f32| {
            arc.lock().update_field(AppContextField::state_progress(progress));
        })
    }
}

#[async_trait]
pub trait Workload {      
    async fn run(&self) -> Result<(), Error>;           
}