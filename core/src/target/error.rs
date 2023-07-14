use std::io;


#[derive(thiserror::Error, Debug)]
pub enum AppEntryError {
    #[error("{0}")]
    OsError(#[from] OsError)
}

#[derive(thiserror::Error, Debug)]
pub enum CreateSymlinkError {
    #[error("{0}")]
    OsError(#[from] OsError),

    #[error("{0}")]
    IoError(#[from] io::Error)
}

#[derive(thiserror::Error, Debug)]
pub enum OsError {
    #[error("{0}")]
    Other(String)
}