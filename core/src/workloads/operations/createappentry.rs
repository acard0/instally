
use serde::{Deserialize, Serialize};

use crate::{*, definitions::operation::OperationPerformer, helpers::serializer};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateAppEntryOperation {
    name: String
}

impl CreateAppEntryOperation {
    pub fn new(name: &str) -> Self {
        CreateAppEntryOperation {
            name: name.into()
        }
    }

    pub fn new_from_weak_struct(package: Option<crate::definitions::package::Package>, weak_struct: &str) -> Result<Self, rust_i18n::error::Error> {
        let next: CreateAppEntryOperation = serializer::from_json(weak_struct)?;
        Ok(next)
    }
}

impl OperationPerformer for CreateAppEntryOperation {
    fn from_record(package: Option<crate::definitions::package::Package>, record: &crate::definitions::operation::OperationRecord) -> Result<Self, rust_i18n::error::Error> where Self: Sized {
        Self::new_from_weak_struct(package, record.get_data())
    }

    fn execute(&mut self, app: &crate::definitions::app::InstallyApp) -> Result<(), rust_i18n::error::Error> {
        crate::sys::create_app_entry(app, "maintenancetool")?;
        Ok(())
    }

    fn finalize(&mut self, app: &crate::definitions::app::InstallyApp) -> Result<(), rust_i18n::error::Error> {
        Ok(())
    }

    fn revert(&mut self, app: &crate::definitions::app::InstallyApp) -> Result<(), rust_i18n::error::Error> {
        crate::sys::delete_app_entry(app)?;
        Ok(())
    }

    fn description(&self) -> String {
        t!("actions.delete-app-entry-operation")
    }

    fn get_kind(&self) -> crate::definitions::operation::OperationKind {
        crate::definitions::operation::OperationKind::CreateAppEntryOperation
    }

    fn as_weak_struct(&self) -> Result<String, serializer::SerializationError> {
        Ok(serializer::to_json(&self)?)
    }
}