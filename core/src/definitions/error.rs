use crate::*;
use crate::{http::client::HttpStreamError, scripting::error::IJSError, helpers::serializer::SerializationError};

use helpers::file::IoError;
use rust_i18n::error::*;
use convert_case::*;

#[derive(thiserror::Error, rust_i18n::AsDetails, strum::AsRefStr, Debug)]
pub enum AppBuildError {
    #[error("{}", .0.get_message_key())]
    Repository(#[from] RepositoryFetchError),

    #[error("{}", .0.get_message_key())]
    Serialization(#[from] SerializationError),

    #[error("{}", .0.get_message_key())]
    Io(#[from] IoError),
}

#[derive(thiserror::Error, rust_i18n::AsDetails, strum::AsRefStr, Debug)]
pub enum RepositoryFetchError {
    #[error("{}", .0.get_message_key())]
    HttpStream(#[from] HttpStreamError),

    #[error("{}", .0.get_message_key())]
    Serialization(#[from] SerializationError),
}

#[derive(thiserror::Error, rust_i18n::AsDetails, strum::AsRefStr, Debug)]
pub enum PackageDownloadError {
    #[error("{}", .0.get_message_key())]
    HttpStream(#[from] HttpStreamError),

    #[error("{}", .0.get_message_key())]
    Io(#[from] IoError),
}

#[derive(thiserror::Error, rust_i18n::AsDetails, strum::AsRefStr, Debug)]
pub enum ScriptError {
    #[error("{}", .0.get_message_key())]
    HttpStream(#[from] HttpStreamError),

    #[error("{}", .0.get_message_key())]
    IJS(#[from] IJSError),
}

#[derive(thiserror::Error, rust_i18n::AsDetails, strum::AsRefStr, Debug)]
pub enum PackageInstallError {
    #[error("{}", .0.get_message_key())]
    Script(#[from] ScriptError),

    #[error("{}", .0.get_message_key())]
    Package(#[from] PackageDownloadError), 

    #[error("{}", .0.get_details().fullname)]
    Other(#[from] rust_i18n::error::Error)
}

#[derive(thiserror::Error, rust_i18n::AsDetails, strum::AsRefStr, Debug)]
pub enum PackageUninstallError {
    #[error("installition-not-found")]
    InstallationNotFound,

    #[error("{}", .0.get_message_key())]
    Script(#[from] ScriptError),

    #[error("{}", .0.get_details().fullname)]
    Other(#[from] rust_i18n::error::Error)
}

#[derive(thiserror::Error, rust_i18n::AsDetails, strum::AsRefStr, Debug)]
pub enum PackageUpdateError {
    #[error("{}", .0.get_message_key())]
    Script(#[from] ScriptError),

    #[error("{}", .0.get_message_key())]
    Package(#[from] PackageDownloadError), 

    #[error("{}", .0.get_message_key())]
    Uninstall(#[from] PackageUninstallError),

    #[error("{}", .0.get_message_key())]
    Install(#[from] PackageInstallError),
}