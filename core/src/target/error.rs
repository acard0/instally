
use std::io;
use crate::{*, error::*};

#[derive(thiserror::Error, struct_field::AsDetails, strum::AsRefStr, Debug)]
pub enum AppEntryError {
    #[error("{}", .0.get_message_key())]
    OsError(#[from] OsError),

    #[error("{}", .0.get_message_key())]
    IoError(#[from] io::Error)
}

#[derive(thiserror::Error, struct_field::AsDetails, strum::AsRefStr, Debug)]
pub enum SymlinkError {
    #[error("{}", .0.get_message_key())]
    OsError(#[from] OsError),

    #[error("{}", .0.get_message_key())]
    IoError(#[from] io::Error)
}

#[derive(thiserror::Error, struct_field::AsDetails, strum::AsRefStr, Debug)]
pub enum OsError {
    #[error("{0}")]
    Other(String)
}