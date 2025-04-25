
use std::path::Path;

use directories::UserDirs;
use serde::{Deserialize, Serialize};

use crate::{helpers::{self, formatter::TemplateFormat, serializer::{self, SerializationError}, workflow::{self, Workflow}}, http::client};

use super::{error::{RepositoryFetchError, ScriptError}, package::Package, repository::Repository};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Product {
    pub name: String,
    pub title: String,
    pub publisher: String,
    pub product_url: String,
    pub repository: String,
    pub script: String,
    pub target_directory: String,
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
        let template: Product = serializer::from_json_file(path)?;
        Ok(template)
    }

    pub fn read() -> Result<Product, SerializationError> {
        Self::read_file(Path::new("product.json"))
    }

    pub fn read_file<P: AsRef<Path>>(path: P) -> Result<Product, SerializationError> {
        let template: Product = serializer::from_json_file(path)?;
        Self::from_template(template)
    }

    pub fn from_template(template: Product) -> Result<Product, SerializationError> {
        let formatter = template.create_formatter();
        let back_step = serializer::to_json(&template)?;
        let json = formatter.format(&back_step);
        Ok(serializer::from_json(&json)?)
    }

    pub fn create_formatter(&self) -> TemplateFormat {
        let directories = UserDirs::new().unwrap(); 

        // use transformer to ensure its valid to be stored as json
        TemplateFormat::new(Some(Box::new(|value| 
                serializer::to_json(&value) // serialize to make sure it can be stored in json file, without breaking its format
                    .and_then(|transformed| Ok(transformed[1..transformed.len()-1].to_owned())) // remove quotes
                    .unwrap_or(value.to_string())) // or get value itself
            ))
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

    pub fn get_uri_to_package_sha1(&self, package: &Package) -> String {
        format!("{}packages/{}.sha1", self.repository, package.archive)
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
        std::path::Path::new(&self.target_directory).join("product.json")
    }

    pub fn get_path_to_self_struct_local(&self) -> std::path::PathBuf {
        std::env::current_dir().unwrap().join("product.json")
    }

    pub fn get_relative_target_directory(&self) -> std::path::PathBuf {
        match workflow::get_workflow_from_env() {
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

    pub(super) async fn fetch_repository(&self) -> Result<Repository, RepositoryFetchError> {
        let meta_uri = format!("{}repository.json", &self.repository);
        let mut meta_str = client::get_text(&meta_uri, |_| ()).await?;
        meta_str = self.create_formatter().format(&meta_str);

        let repository: Repository = serializer::from_json(&meta_str)?;

        log::info!("Fetched and parsed Repository structure for {}", self.name);
        Ok(repository)
    }

    pub(super) fn dump(&self) -> Result<(), SerializationError> {
        let mut file = helpers::file::open_create(&self.get_path_to_self_struct_target())?;
        helpers::file::write_all_file(&mut file, serializer::to_json(self)?.as_bytes())?;
        Ok(())
    }
}