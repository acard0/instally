use crate::*;
use crate::{http::client::HttpStreamError, scripting::error::IJSError, helpers::serializer::SerializationError};

use rust_i18n::error::*;
use convert_case::*;

#[derive(thiserror::Error, rust_i18n::AsDetails, strum::AsRefStr, Debug)]
pub enum RepositoryFetchError {
    #[error("http-stream.{}", .0.get_display_key())]
    HttpStream(#[from] HttpStreamError),

    #[error("serialization.{}", .0.get_display_key())]
    Serialization(#[from] SerializationError),
}

#[derive(thiserror::Error, rust_i18n::AsDetails, strum::AsRefStr, Debug)]
pub enum PackageDownloadError {
    #[error("http-stream.{}", .0.get_display_key())]
    HttpStream(#[from] HttpStreamError),

    #[error("io.{}", .0.kind().to_string().to_case(Case::Kebab))]
    Io(#[from] std::io::Error),
}

#[derive(thiserror::Error, rust_i18n::AsDetails, strum::AsRefStr, Debug)]
pub enum ScriptError {
    #[error("http-stream.{}", .0.get_display_key())]
    HttpStream(#[from] HttpStreamError),

    #[error("ijs.{}", .0.get_display_key())]
    IJS(#[from] IJSError),
}

#[derive(thiserror::Error, rust_i18n::AsDetails, strum::AsRefStr, Debug)]
pub enum PackageInstallError {
    #[error("archive")]
    Archive(#[from] zip::result::ZipError),
    
    #[error("serialization.{}", .0.get_display_key())]
    Serialization(#[from] SerializationError)
}

#[derive(thiserror::Error, rust_i18n::AsDetails, strum::AsRefStr, Debug)]
pub enum PackageUninstallError {
    #[error("installition-not-found")]
    InstallitionNotFound,

    #[error("io.{}", .0.kind().to_string().to_case(Case::Kebab))]
    Io(#[from] std::io::Error),

    #[error("serialization.{}", .0.get_display_key())]
    Serialization(#[from] SerializationError),
}