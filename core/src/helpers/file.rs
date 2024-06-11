use crate::*;

use std::{fs::{File, OpenOptions}, io::{Read, Write}, path::{Path, PathBuf}};
use convert_case::{Case, Casing};

use filepath::FilePath;
use rust_i18n::error::*;

#[derive(thiserror::Error, rust_i18n::AsDetails, Debug)]
#[error("io-error.{}", .0.kind().to_string().to_case(Case::Kebab))]
pub struct IoError(#[from] std::io::Error);

pub fn cwd() -> Result<PathBuf, IoError> {
    let cwd = std::env::current_exe()?;
    let canonicalized = std::fs::canonicalize(&cwd)?;

    Ok(strip_extended_length_prefix(&canonicalized).parent().ok_or_else(|| IoError::from(std::io::Error::from(std::io::ErrorKind::NotFound)))?.to_path_buf())
}

pub fn open<P: AsRef<Path>>(path: P) -> Result<File, IoError> {
    let file = OpenOptions::new()
        .read(true).write(true).open(path.as_ref())?;

    Ok(file)
}

pub fn open_create<P: AsRef<Path>>(path: P) -> Result<File, IoError> {
    let file = OpenOptions::new()
        .create(true).read(true).write(true).open(path.as_ref())?;
    
    Ok(file)
}

pub fn create<P: AsRef<Path>>(path: P) -> Result<File, IoError> {
    let file = OpenOptions::new()
        .create(true).truncate(true).write(true).read(true).open(path.as_ref())?;
    
    Ok(file)
}

pub fn copy_file<P: AsRef<Path>>(from: P, to: P) -> Result<u64, IoError> {
    Ok(std::fs::copy(from, to)?)
}

pub fn copy_stream<R: ?Sized + Read, W: ?Sized + Write>(reader: &mut R, writer: &mut W) -> Result<u64, IoError> {
    Ok(std::io::copy(reader, writer)?)
}

pub fn delete<P: AsRef<Path>>(from: P) -> Result<(), IoError> {
    Ok(std::fs::remove_file(from)?)
}

pub fn read_to_string<P: AsRef<Path>>(path: P) -> Result<String, IoError> {
    Ok(std::fs::read_to_string(path)?)
}

pub fn write_all<P: AsRef<Path>>(path: P, buffer: &[u8]) -> Result<(), IoError>{
    Ok(write_all_file(&mut create(path)?, buffer)?)
}

pub fn write_all_stream(writer: &mut impl Write, buffer: &[u8]) -> Result<(), IoError> {
    Ok(std::io::Write::write_all(writer, buffer)?)
}

pub fn write_all_file(file: &mut File, buf: &[u8]) -> Result<(), IoError>{
    Ok(file.write_all(buf)?)
}

pub fn create_dir_all<P: AsRef<Path>>(path: P) ->  Result<(), IoError>{
    Ok(std::fs::create_dir_all(path)?)
}

pub fn read(file: &mut File, buffer: &mut [u8]) -> Result<usize, IoError> {
    Ok(file.read(buffer)?)
}

pub fn read_to_end(file: &mut File, buffer: &mut Vec<u8>) -> Result<usize, IoError> {
    Ok(file.read_to_end(buffer)?)
}

pub fn read_to_string_from_file(file: &mut File, buf:  &mut String) -> Result<usize, IoError> {
    Ok(file.read_to_string(buf)?)
}

pub fn path(file: &mut File) -> Result<PathBuf, IoError> {
    Ok(file.path()?)
}

fn strip_extended_length_prefix(path: &Path) -> PathBuf {
    const VERBATIM_PREFIX: &str = r"\\?\";
    let path_str = path.to_str().unwrap_or_default();
    if path_str.starts_with(VERBATIM_PREFIX) {
        PathBuf::from(&path_str[VERBATIM_PREFIX.len()..])
    } else {
        path.to_path_buf()
    }
}