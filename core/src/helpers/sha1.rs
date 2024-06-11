use std::fs::File;

use convert_case::*;
use rust_i18n::error::*;
use sha1::{Sha1, Digest};

use crate::{*, helpers};

use super::file::IoError;

#[derive(thiserror::Error, rust_i18n::AsDetails, strum::AsRefStr, Debug)]
pub enum Sha1Error {
    #[error("io-error")]
    Io(#[from] IoError),
}

pub fn generate_sha1<P: AsRef<std::path::Path>>(path: P) -> Result<String, Sha1Error> {
    let mut file = helpers::file::open(path)?;
    generate_sha1_file(&mut file)
}

pub fn generate_sha1_file(file: &mut File) -> Result<String, Sha1Error> {
    let mut hasher = Sha1::new();
    let mut buffer = [0; 1024];

    loop {
        let size = helpers::file::read(file, &mut buffer)?;

        if size == 0 {
            break;
        }
        hasher.update(&buffer[..size]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

pub fn verify_sha1<P: AsRef<std::path::Path>>(path: P, compare: &str) -> Result<bool, Sha1Error> {
    Ok(generate_sha1(path)? == compare)
}

pub fn verify_sha1_file(file: &mut File, compare: &str, sha1: Option<&mut String>) -> Result<bool, Sha1Error> {
    let check: String = generate_sha1_file(file)?;
    
    if let Some(out) = sha1 {
        *out = check.clone();
    }

    Ok(check == compare)
}

pub fn writeout_sha1<P: AsRef<std::path::Path>>(path: P) -> Result<(), Sha1Error> {
    Ok(helpers::file::write_all(path.as_ref(), generate_sha1(path.as_ref())?.as_bytes())?)
}