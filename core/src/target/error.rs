
use std::io;
use crate::{*, error::*};

#[derive(thiserror::Error, struct_field::AsError, strum::AsRefStr, Debug)]
pub enum AppEntryError {
    #[error("Failed to modify app entry {0}")]
    OsError(#[from] OsError),

    #[error("Failed to modify app entry {0}")]
    IoError(#[from] io::Error)
}

#[derive(thiserror::Error, struct_field::AsError, strum::AsRefStr, Debug)]
pub enum SymlinkError {
    #[error("Failed to perform symlink operation {0}")]
    OsError(#[from] OsError),

    #[error("Failed to perform symlink operation {0}")]
    IoError(#[from] io::Error)
}

#[derive(thiserror::Error, struct_field::AsError, strum::AsRefStr, Debug)]
pub enum OsError {
    #[error("{0}")]
    Other(String)
}