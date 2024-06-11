
use serde::{Deserialize, Serialize};

use crate::{*, definitions::operation::OperationPerformer, helpers::{self, serializer}};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateFileOperation {
    destination: std::path::PathBuf
}

impl CreateFileOperation {
    pub fn new(path: &std::path::PathBuf) -> Self {
        CreateFileOperation {
            destination: path.into()
        }
    }

    pub fn new_from_weak_struct(package: Option<crate::definitions::package::Package>, weak_struct: &str) -> Result<Self, rust_i18n::error::Error> {
        let next: CreateFileOperation = serializer::from_json(weak_struct)?;
        Ok(next)
    }
}

impl OperationPerformer for CreateFileOperation {
    fn from_record(package: Option<crate::definitions::package::Package>, record: &crate::definitions::operation::OperationRecord) -> Result<Self, rust_i18n::error::Error> where Self: Sized {
        Self::new_from_weak_struct(package, record.get_data())
    }

    fn execute(&mut self, app: &crate::definitions::app::InstallyApp) -> Result<(), rust_i18n::error::Error> {
        helpers::file::open_create(&self.destination)?;
        Ok(())
    }

    fn finalize(&mut self, app: &crate::definitions::app::InstallyApp) -> Result<(), rust_i18n::error::Error> {
        Ok(())
    }

    fn revert(&mut self, app: &crate::definitions::app::InstallyApp) -> Result<(), rust_i18n::error::Error> {
        helpers::file::delete(&self.destination)?;
        Ok(())
    }

    fn description(&self) -> String {
        t!("actions.create-file-operation")
    }

    fn get_kind(&self) -> crate::definitions::operation::OperationKind {
        crate::definitions::operation::OperationKind::CreateFileOperation
    }

    fn as_weak_struct(&self) -> Result<String, serializer::SerializationError> {
        Ok(serializer::to_json(&self)?)
    }
}