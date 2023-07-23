
use crate::{http::client::HttpStreamError, scripting::error::IJSError, helpers::serializer::{SerializationError, self}};
use crate::{*, error::*};

#[derive(thiserror::Error, struct_field::AsDetails, strum::AsRefStr, Debug)]
pub enum RepositoryFetchError {
    #[error("{}", .0.get_message_key())]
    NetworkError(#[from] HttpStreamError),

    #[error("{}", .0.get_message_key())]
    ParseError(#[from] serializer::SerializationError),
}

#[derive(thiserror::Error, struct_field::AsDetails, strum::AsRefStr, Debug)]
pub enum WeakStructParseError {
    #[error("{}", .0.get_message_key())]
    IOError(#[from] std::io::Error),
    
    #[error("{}", .0.get_message_key())]
    ParseError(#[from] SerializationError)
}   

#[derive(thiserror::Error, struct_field::AsDetails, strum::AsRefStr, Debug)]
pub enum PackageDownloadError {
    #[error("{}", .0.get_message_key())]
    NetworkError(#[from] HttpStreamError),

    #[error("{}", .0.get_message_key())]
    IOError(#[from] std::io::Error)
}

#[derive(thiserror::Error, struct_field::AsDetails, strum::AsRefStr, Debug)]
pub enum ScriptError {
    #[error("{}", .0.get_message_key())]
    HttpStreamError(#[from] HttpStreamError),

    #[error("{}", .0.get_message_key())]
    IOError(#[from] std::io::Error),

    #[error("{}", .0.get_message_key())]
    IJSError(#[from] IJSError),

    #[error("{0}")]
    Other(String)
}

#[derive(thiserror::Error, struct_field::AsDetails, strum::AsRefStr, Debug)]
pub enum PackageInstallError {
    #[error("{}", .0.as_details())]
    IOError(#[from] std::io::Error),

    #[error("archive-error")]
    ArchiveError(#[from] zip::result::ZipError),
    
    #[error("{}", .0.as_details())]
    SummaryIOError(#[from] WeakStructParseError)
}

#[derive(thiserror::Error, struct_field::AsDetails, strum::AsRefStr, Debug)]
pub enum PackageUninstallError {
    #[error("installition-not-found")]
    InstallitionNotFound,

    #[error("{}", .0.as_details())]
    IOError(#[from] std::io::Error),

    #[error("{}", .0.as_details())]
    SummaryIOError(#[from] WeakStructParseError)
}

#[derive(thiserror::Error, struct_field::AsDetails, strum::AsRefStr, Debug)]
pub enum RepositoryCrossCheckError {
    #[error("{}", .0.as_details())]
    FailedToFetchRemoteTree(#[from] RepositoryFetchError),

    #[error("{}", .0.as_details())]
    FailedToParseInstallitionSummary(#[from] WeakStructParseError)
}
