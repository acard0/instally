
use serde::{Deserialize, Serialize};

use crate::{*, definitions::operation::OperationPerformer, helpers::serializer};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateSymlinkOperation {
    original: std::path::PathBuf,
    destination: std::path::PathBuf,
    link_name: String
}

impl CreateSymlinkOperation {
    pub fn new<P: AsRef<std::path::Path>>(original: P, destination: P, link_name: &str) -> Self {
        CreateSymlinkOperation {
            original: original.as_ref().into(),
            destination: destination.as_ref().into(),
            link_name: link_name.into(),
        }
    }

    pub fn new_from_weak_struct(package: Option<crate::definitions::package::Package>, weak_struct: &str) -> Result<Self, rust_i18n::error::Error> {
        let next: CreateSymlinkOperation = serializer::from_json(weak_struct)?;
        Ok(next)
    }
}

impl OperationPerformer for CreateSymlinkOperation {
    fn from_record(package: Option<crate::definitions::package::Package>, record: &crate::definitions::operation::OperationRecord) -> Result<Self, rust_i18n::error::Error> where Self: Sized {
        Self::new_from_weak_struct(package, record.get_data())
    }

    fn execute(&mut self, app: &crate::definitions::app::InstallyApp) -> Result<(), rust_i18n::error::Error> {
        crate::sys::symlink_file(&self.original, &self.destination, &self.link_name)?;
        Ok(())
    }

    fn finalize(&mut self, app: &crate::definitions::app::InstallyApp) -> Result<(), rust_i18n::error::Error> {
        Ok(())
    }

    fn revert(&mut self, app: &crate::definitions::app::InstallyApp) -> Result<(), rust_i18n::error::Error> {
        crate::sys::break_symlink_file(&self.destination, &self.link_name)?;
        Ok(())
    }

    fn description(&self) -> String {
        t!("actions.create-symlink-operation")
    }

    fn get_kind(&self) -> crate::definitions::operation::OperationKind {
        crate::definitions::operation::OperationKind::CreateSymlinkOperation
    }

    fn as_weak_struct(&self) -> Result<String, serializer::SerializationError> {
        Ok(serializer::to_json(&self)?)
    }
}