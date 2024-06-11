use std::fmt::{Display, Formatter};

use async_trait::async_trait;
use rust_i18n::error::ErrorDetails;

#[async_trait]
pub trait Workload {      
    async fn run(&mut self) -> Result<(), rust_i18n::error::Error>;           
    async fn finalize(&mut self, has_error: bool) -> Result<(), rust_i18n::error::Error>;
}

#[derive(Debug, Clone)]
pub enum WorkloadResult {
    Ok,
    Error(ErrorDetails)
}

impl Display for WorkloadResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error(err) => write!(f, "{err:?}"),
            _ => write!(f, "Ok")
        }
    }
}

impl WorkloadResult {
    pub fn is_ok(&self) -> bool {
        match self {
            Self::Ok => true,
            _ => false
        }
    }

    pub fn get_error(&self) -> Option<ErrorDetails> {
        match self {
            Self::Error(err) => Some(err.clone()),
            _ => None,
        }
    }
}
