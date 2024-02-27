use crate::*;
use std::{io::ErrorKind, path::{Path, PathBuf}};
use convert_case::{Case, Casing};
use rust_i18n::error::*;

#[derive(thiserror::Error, rust_i18n::AsDetails, Debug)]
pub struct IoError {
    pub kind: ErrorKind
}

impl From<ErrorKind> for IoError {
    fn from(value: ErrorKind) -> Self {
        IoError {
            kind: value
        }
    }
}

impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind.to_string().to_case(Case::Kebab))
    }
}

impl From<std::io::Error> for IoError {
    fn from(value: std::io::Error) -> Self {
        IoError {
            kind: value.kind()
        }
    }
}

pub fn open<P: Into<PathBuf>>(path: P) -> std::io::Result<fs_err::File> {
    let file = fs_err::OpenOptions::new()
        .read(true).write(true).open(path)?;

    Ok(file)
}

pub fn open_create<P: Into<PathBuf>>(path: P) -> std::io::Result<fs_err::File> {
    let file = fs_err::OpenOptions::new()
        .create(true).read(true).write(true).open(path)?;
    
    Ok(file)
}

pub fn copy<P, Q>(from: P, to: Q) -> std::io::Result<u64>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    fs_err::copy(from, to)
}

pub fn create_dir_all<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    fs_err::create_dir_all(path)
}
