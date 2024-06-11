use std::fmt::{Formatter, Display};

use async_trait::async_trait;
use rust_i18n::error::Error;

use crate::*;
use crate::definitions::context::AppWrapper;
use super::workload::Workload;

pub type NoopWrapper = AppWrapper<NoopOptions>;

#[derive(Clone)]
pub struct NoopOptions;

impl Default for NoopOptions {
    fn default() -> Self {
        NoopOptions { }
    }
}

#[async_trait] 
impl Workload for NoopWrapper {
    async fn run(&mut self) -> Result<(), Error> {
        Ok(())
    }
    
    async fn finalize(&mut self, has_error: bool) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum NoopWorkloadState {
    Done,
}

impl Display for NoopWorkloadState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            _ => write!(f, "{:?}", t!("states.completed"))
        }
    }
}

impl Default for NoopWorkloadState {
    fn default() -> Self {
        Self::Done
    }
}