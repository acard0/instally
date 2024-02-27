
use rust_i18n::error::*;
use convert_case::*;
use crate::*;

#[derive(thiserror::Error, rust_i18n::AsDetails, strum::AsRefStr, Debug)]
pub enum AppEntryError {
    #[error("os.{}", .0.get_display_key())]
    Os(#[from] OsError),

    #[error("io.{}", .0.kind().to_string().to_case(Case::Kebab))]
    Io(#[from] std::io::Error)
}

#[derive(thiserror::Error, rust_i18n::AsDetails, strum::AsRefStr, Debug)]
pub enum SymlinkError {
    #[error("os.{}", .0.get_display_key())]
    Os(#[from] OsError),

    #[error("io.{}", .0.kind().to_string().to_case(Case::Kebab))]
    Io(#[from] std::io::Error)
}

#[derive(thiserror::Error, rust_i18n::AsDetails, strum::AsRefStr, Debug)]
pub enum OsError {
    #[error("other")]
    Other(String)
}