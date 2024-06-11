
use serde::{Deserialize, Serialize};

use crate::{definitions::operation::OperationPerformer, helpers::{file::IoError, serializer}, *};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateMaintenanceToolOperation {
    name: String
}

impl CreateMaintenanceToolOperation {
    pub fn new(name: &str) -> Self {
        CreateMaintenanceToolOperation {
            name: name.into()
        }
    }

    pub fn new_from_weak_struct(package: Option<crate::definitions::package::Package>, weak_struct: &str) -> Result<Self, rust_i18n::error::Error> {
        let next: CreateMaintenanceToolOperation = serializer::from_json(weak_struct)?;
        Ok(next)
    }
}

impl OperationPerformer for CreateMaintenanceToolOperation {
    fn from_record(package: Option<crate::definitions::package::Package>, record: &crate::definitions::operation::OperationRecord) -> Result<Self, rust_i18n::error::Error> where Self: Sized {
        Self::new_from_weak_struct(package, record.get_data())
    }

    fn execute(&mut self, app: &crate::definitions::app::InstallyApp) -> Result<(), rust_i18n::error::Error> {
        crate::sys::create_maintenance_tool(app, &self.name)?;
        Ok(())
    }

    fn finalize(&mut self, app: &crate::definitions::app::InstallyApp) -> Result<(), rust_i18n::error::Error> {
        Ok(())
    }

    fn revert(&mut self, app: &crate::definitions::app::InstallyApp) -> Result<(), rust_i18n::error::Error> {
        // win: schelude binary for deletion, unix: delete in place
        self_replace::self_delete().map_err(|err| IoError::from(err))?;
        Ok(())
    }

    fn description(&self) -> String {
        t!("actions.create-maintenancetool-operation")
    }

    fn get_kind(&self) -> crate::definitions::operation::OperationKind {
        crate::definitions::operation::OperationKind::CreateMaintenanceToolOperation
    }

    fn as_weak_struct(&self) -> Result<String, serializer::SerializationError> {
        Ok(serializer::to_json(&self)?)
    }
}