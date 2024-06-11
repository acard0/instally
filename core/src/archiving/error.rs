use convert_case::*;
use helpers::{file::IoError, sha1::Sha1Error};
use zip::result::ZipError;
use rust_i18n::error::*;

use crate::*;

#[derive(thiserror::Error, rust_i18n::AsDetails, strum::AsRefStr, Debug)]
pub enum ArchiveError {
    #[error("{}", .0.get_message_key())]
    Io(#[from] IoError),

    #[error("invalid")]
    InvalidArchive,
    
    #[error("unsupported")]
    UnknownArchive,

    #[error("invalid-password")]
    Password,

    #[error("mismatching-sha1")]
    Sha1Mismatch,

    #[error("{}", .0.get_message_key())]
    Sha1(#[from] Sha1Error)
}

impl From<ZipError> for ArchiveError {
    fn from(value: ZipError) -> Self {
        match value {
            ZipError::FileNotFound => {
                ArchiveError::Io(std::io::Error::from(std::io::ErrorKind::NotFound).into())
            },
            ZipError::InvalidArchive(err) => {
                ArchiveError::InvalidArchive
            },
            ZipError::Io(err) => {
                ArchiveError::Io(err.into())
            },
            ZipError::UnsupportedArchive(err) => {
                ArchiveError::UnknownArchive
            },
            ZipError::InvalidPassword => {
                ArchiveError::Password
            }
            _ => todo!(),
        }
    }
}

#[test]
fn test() {
    let err = ArchiveError::Io(IoError::from(std::io::Error::from(std::io::ErrorKind::NotFound)));
    println!("{:?}", err.as_details());
}