use crate::http::client::HttpStreamError;



#[derive(thiserror::Error, Debug)]
pub enum WorkloadError {
    #[error("{0}")]
    Other(String)
}

#[derive(thiserror::Error, Debug)]
pub enum RepositoryFetchError {
    #[error("A error accured while pulling remote repository. {0}")]
    NetworkError(#[from] HttpStreamError),

    #[error("A error accured while parsing remote repository structure. {0}")]
    ParseError(#[from] serde_xml_rs::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum WeakStructParseError {

    #[error("IO Error accured while pulling weak structure from file. {0}")]
    IOError(#[from] std::io::Error),

    #[error("An error accured while parsing weak structure from file. {0}")]
    ParseError(#[from] serde_xml_rs::Error)
}

#[derive(thiserror::Error, Debug)]
pub enum PackageDownloadError {

    #[error("A error accured while pulling package from repository. {0}")]
    NetworkError(#[from] HttpStreamError),

    #[error("An error accured while downloading a package. {0}")]
    IOError(#[from] std::io::Error)
}


#[derive(thiserror::Error, Debug)]
pub enum PackageInstallError {
    #[error("An error accured while reading package file. {0}")]
    IOError(#[from] std::io::Error),

    #[error("An error accured while unpacking package file. {0}")]
    ArchiveError(#[from] zip::result::ZipError)
}