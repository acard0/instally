
use crate::{http::client::HttpStreamError, scripting::error::IJSError, helpers::serializer::{SerializationError, self}};
use crate::{*, error::*};

#[derive(thiserror::Error, struct_field::AsError, strum::AsRefStr, Debug)]
pub enum RepositoryFetchError {
    #[error("Failed to pull remote repository structure {0}")]
    NetworkError(#[from] HttpStreamError),

    #[error("Serialization error accured while attempting to parse repository weak structure {0}")]
    ParseError(#[from] serializer::SerializationError),
}

#[derive(thiserror::Error, struct_field::AsError, strum::AsRefStr, Debug)]
pub enum WeakStructParseError {
    #[error("IO error accured while trying parse a weak structure {0}")]
    IOError(#[from] std::io::Error),
    
    #[error(transparent)]
    ParseError(#[from] SerializationError)
}

#[derive(thiserror::Error, struct_field::AsError, strum::AsRefStr, Debug)]
pub enum PackageDownloadError {

    #[error("Was not able to pull package due to http stream error {0}")]
    NetworkError(#[from] HttpStreamError),

    #[error("Was not able to write package due to io error {0}")]
    IOError(#[from] std::io::Error)
}

#[derive(thiserror::Error, struct_field::AsError, strum::AsRefStr, Debug)]
pub enum ScriptError {
    #[error("Attempted to pull the script from net but {0}")]
    HttpStreamError(#[from] HttpStreamError),

    #[error("Attempted to parse the script from file but {0}")]
    IOError(#[from] std::io::Error),

    #[error("Attempted to evaluate the script but {0}")]
    IJSError(#[from] IJSError),

    #[error("{0}")]
    Other(String)
}

#[derive(thiserror::Error, struct_field::AsError, strum::AsRefStr, Debug)]
pub enum PackageInstallError {
    #[error("An error accured while reading package file. {0}")]
    IOError(#[from] std::io::Error),

    #[error("An error accured while unpacking package file. {0}")]
    ArchiveError(#[from] zip::result::ZipError),

    #[error("An error accured while accessing to installition summary file. {0}")]
    SummaryIOError(#[from] WeakStructParseError)
}

#[derive(thiserror::Error, struct_field::AsError, strum::AsRefStr, Debug)]
pub enum PackageUninstallError {
    #[error("Package is not installed.")]
    InstallitionNotFound,

    #[error("An error accured while removing files. {0}")]
    IOError(#[from] std::io::Error),

    #[error("An error accured while accessing to installition summary file. {0}")]
    SummaryIOError(#[from] WeakStructParseError)
}

#[derive(thiserror::Error, struct_field::AsError, strum::AsRefStr, Debug)]
pub enum RepositoryCrossCheckError {
 
    #[error("Failed to get remote tree. {0}")]
    FailedToFetchRemoteTree(#[from] RepositoryFetchError),

    #[error("Failed to get installition summary. {0}")]
    FailedToParseInstallitionSummary(#[from] WeakStructParseError)
}
