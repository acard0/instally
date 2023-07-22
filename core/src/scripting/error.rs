
use crate::{*, error::*};

#[derive(thiserror::Error, struct_field::AsError, strum::AsRefStr, Debug)]
pub enum IJSError {
    #[error("{0}")]
    Other(String),
}