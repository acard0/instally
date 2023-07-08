use std::io;


#[derive(thiserror::Error, Debug)]
pub enum CreateAppEntryError {
    #[error("{0}")]
    OsError(String)
}

#[derive(thiserror::Error, Debug)]
pub enum CreateSymlinkError {
    #[error("{0}")]
    OsError(String),

    #[error("{0}")]
    IoError(#[from] io::Error)
}