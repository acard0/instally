use std::{fmt::Display, path::Path, sync::Arc};

use parking_lot::Mutex;

use crate::{definitions::{dependency::{DependencyFile, PackageFile}, package::Package, product::Product, repository::Repository, script::Script, summary::InstallationSummary}, helpers::{self, file::IoError, tmp, workflow::Workflow}, http::client::{self, HttpStreamError}, workloads::{operations::{archive::ExtractArchiveOperation, createappentry::CreateAppEntryOperation, createfile::CreateFileOperation, createmaintinancetool::CreateMaintenanceToolOperation, createsymlink::CreateSymlinkOperation}, workload::WorkloadResult}};

use super::{context::{AppContext, AppContextField}, error::{AppBuildError, PackageDownloadError, PackageInstallError, PackageUninstallError, RepositoryFetchError, ScriptError}, operation::{Operation, OperationHistory}, script::ScriptOptional, summary::PackageInstallation};

#[derive(Clone, Debug)]
pub struct InstallyApp {
    product: Product,
    repository: Repository,
    context: Arc<Mutex<AppContext>>,
}

impl Default for InstallyApp {
    fn default() -> Self {
        Self { product: Default::default(), repository: Default::default(), context: Arc::new(Mutex::new(AppContext::default())) }
    }
}

impl InstallyApp
{
    pub async fn build(product: &Product) -> Result<Self, AppBuildError> {
        log::info!("Building InstallyApp meta");

        let workflow = helpers::workflow::define_workflow_env(&product)?;

        let summary = match workflow {
            Workflow::FreshInstallition => {
                InstallationSummary::default(&product)
            }
            _ => {
                InstallationSummary::read()?
            }
        };
        
        if workflow != Workflow::FfiApi {
            helpers::process::terminate_processes_under_folder(&product.get_relative_target_directory())
                .expect("Failed to terminate processes under the target directory!");
        }

        Ok(InstallyApp {
            context: Arc::new(Mutex::new(AppContext::new(summary))),
            product: product.clone(), 
            repository: match product.fetch_repository().await {
                Ok(it) => it,
                Err(err) => return Err(err.into()),
            },
        })
    }

    /// Gets app context
    pub fn get_context(&self) -> Arc<Mutex<AppContext>> {
        self.context.clone()
    }

    /// Gets installation summary
    pub fn get_summary(&self) -> InstallationSummary {
        self.get_context().lock().get_summary() 
    }

    /// Modifies the summary without moving it
    pub fn modify_summary<F, Out>(&self, operation: F) -> Out
    where
        F: FnOnce(&mut InstallationSummary) -> Out,
    {
        let binding = self.get_context();
        let mut context = binding.lock();
        let mut summary = context.get_summary_mut();
        let result = operation(&mut summary);

        result
    }

    /// Gets 'Product'
    pub fn get_product(&self) -> &Product {
        &self.product
    }

    /// Gets 'Repository'
    pub fn get_repository(&self) -> &Repository {
        &self.repository
    }

    /// Sets workload state
    pub fn set_workload_state<S: Display>(&self, n_state: S) {
        let mut ctx = self.context.lock(); 
        ctx.update_field(AppContextField::state(Some(n_state.to_string())))
    }

    /// Sets workload progress
    pub fn set_state_progress(&self, n_progress: f32) {
        let mut ctx = self.context.lock(); 
        ctx.update_field(AppContextField::state_progress(n_progress));
    }

    /// Gets workload result
    pub fn set_result(&self, result: &WorkloadResult) {
        let mut ctx = self.context.lock(); 
        ctx.update_field(AppContextField::result(Some(result.clone())))
    }

    /// Fetchs remote tree meta
    pub async fn fetch_repository(&self) -> Result<Repository, RepositoryFetchError>{
        self.product.fetch_repository().await
    }

    /// Downloads package file of specified package
    pub async fn download_package(&self, package: &Package) -> Result<PackageFile, PackageDownloadError>{
        let product = &self.product;  
        let mut file = tmp::create_tmp_file().map_err(|err| IoError::from(err))?;
        let _ = self.get_file(&product.get_uri_to_package(package), file.as_file_mut()).await?;
        let sha1 = self.get_text(&product.get_uri_to_package_sha1(package)).await?;
        Ok(PackageFile { handle: Arc::new(Mutex::new(file)), package: package.clone(), sha1 })
    }

    /// Downloads installation script of specified package
    pub async fn download_package_script(&self, package: &Package) -> Result<Option<Script>, ScriptError> {
        self.download_script(self.product.get_uri_to_package_script(package)?, Some(&package)).await
    }

    /// Downloads installation script of product
    pub async fn download_global_script(&self) -> Result<Option<Script>, ScriptError> {
        self.download_script(self.product.get_uri_to_global_script(self.get_repository()), None).await
    }

    /// Downloads installation script
    pub async fn download_script(&self, uri: Option<String>, target_package: Option<&Package>) -> Result<Option<Script>, ScriptError> {
        match uri {
            None => Ok(None),
            Some(uri) => {
                let src = self.get_text(&uri).await?;
                Ok(Some(Script::new(src, self, target_package)?))
            }
        } 
    }

    /// Gets a dependency file from an uri
    pub async fn get_dependency(&self, uri: &str, state_text: &str) -> Result<DependencyFile, PackageDownloadError>{
        self.set_workload_state(state_text);

        let mut file = tmp::create_tmp_file().map_err(|err| IoError::from(err))?;
        let _ = self.get_file(uri, file.as_file_mut()).await?;
        Ok(DependencyFile::new(file))
    }
 
    /// Performs a fresh installation for specified package file
    pub async fn install_package(&self, package_file: &PackageFile) -> Result<(), PackageInstallError> {
        // create package installation meta without persisting it. required as pre-installation operation records needs a history to be saved
        self.modify_summary(|summary| {
            summary.add_package(&package_file.package, OperationHistory::from_operations(vec![]));
        });

        let product = &self.product;
        let script = self.download_package_script(&package_file.package).await?;
        script.if_exist(|s| Ok(s.invoke_before_installition()?))?;

        let mut operations = Vec::<Operation>::new();
        operations.push(Operation::from_performer(Box::new(ExtractArchiveOperation::new(package_file, product.get_path_to_package(&package_file.package).to_str().unwrap()))));
        // ...

        for operation in &mut operations {
            if let Err(err) = operation.execute(self, Some(&package_file.package)) {
                log::error!("Failed to execute '{:?}'. It's included inside {} package, aborting. {}", operation.get_kind(), package_file.package.display_name, err);
                return Err(err.into());
            }
        }

        script.if_exist(|s| Ok(s.invoke_after_installition()?))?;

        Ok(())
    }

    /// Performs uninstallation for specified package installation
    pub async fn uninstall_package(&self, package_installation: &PackageInstallation) -> Result<(), PackageUninstallError> {
        let product = &self.product;
        let package = self.repository.get_package(&package_installation.name).unwrap();
        let script = self.download_package_script(&package).await?; 

        script.if_exist(|s| Ok(s.invoke_before_uninstallition()?))?;

        package_installation.operations.get_records().into_iter().for_each(|record| {
            if let Err(err) = record.into_operation(Some(&package)).and_then(|mut operation| operation.revert(&self, None)) {
                log::error!("Failed to revert operation {:?}, included inside {} package. {:?}", record.get_kind(), package.display_name, err);
            }
        }); 
    
        script.if_exist(|s| Ok(s.invoke_after_uninstallition()?))?;

        self.modify_summary(|summary| {
            summary.remove_package(&package_installation.name).map(|sum| ())
        })?;

        Ok(())
    } 

    /// Downloads the specified file
    pub async fn get_file(&self, url: &str, file: &mut std::fs::File) -> Result<(), HttpStreamError> {
        let progress_closure = self.create_progress_closure();
        client::get_file(url, file, progress_closure).await
    }
    
    /// Gets the specified text
    pub async fn get_text(&self, url: &str) -> Result<String, HttpStreamError> {
        let progress_closure = self.create_progress_closure();
        client::get_text(url, progress_closure).await
    }

    /// Creates a smylink
    pub fn symlink_file<P: AsRef<Path>>(&self, target: Option<&Package>, original: P, link_dir: P, link_name: &str) -> Result<(), rust_i18n::error::Error> {
        let mut operation = Operation::from_performer(Box::new(CreateSymlinkOperation::new(original, link_dir, link_name)));
        operation.execute(self, target)
    }

    /// Creates an app entry
    pub fn create_app_entry(&self, maintenance_tool_name: &str) -> Result<(), rust_i18n::error::Error> {
        let mut operation = Operation::from_performer(Box::new(CreateAppEntryOperation::new(maintenance_tool_name)));
        operation.execute(self, None)
    }

    /// Creates the maintenance tool
    pub fn create_maintenance_tool(&self, maintenance_tool_name: &str) -> Result<(), rust_i18n::error::Error> {
        let mut operation = Operation::from_performer(Box::new(CreateMaintenanceToolOperation::new(maintenance_tool_name)));
        operation.execute(self, None)
    }

    /// Dumps the product meta struct to the disk
    pub fn dump_product_to_installation_directory(&self, target: Option<&Package>) -> Result<(), rust_i18n::error::Error> {
        let mut operation = Operation::from_performer(Box::new(CreateFileOperation::new(&self.product.get_path_to_self_struct_target())));
        operation.execute(self, target)?;

        self.product.dump()?;
        Ok(())
    }

    /// Creates a progress closure
    pub fn create_progress_closure(&self) -> Box<dyn Fn(f32) + Send> {
        let arc = self.get_context(); 
        Box::new(move |progress: f32| {
            arc.lock().update_field(AppContextField::state_progress(progress));
        })
    }

    /// Persists changes made over the summary to the disk.
    pub fn persist_summary(&self) {
        self.modify_summary(|summary| {
            _ = summary.save().expect("Failed to serialize the InstallationSummary while persisting changes to the disk.");
        });
    }
}
