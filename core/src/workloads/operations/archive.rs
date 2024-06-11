use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};
use crate::{archiving::{self, error::ArchiveError}, *};
use self::{definitions::{app::InstallyApp, dependency::PackageFile, operation::{OperationPerformer, OperationRecord}, package::Package}, helpers::serializer};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ExtractArchiveOperationInner {
    destination: String,
    files: Vec<std::path::PathBuf>,
}

pub struct ExtractArchiveOperation<'a> {
    target: Package,
    archive: Option<&'a PackageFile>,
    inner: ExtractArchiveOperationInner,
}

impl<'a> ExtractArchiveOperation<'a> {
    pub fn new(archive: &'a PackageFile, destination: &str) -> Self {
        ExtractArchiveOperation {
            archive: Some(archive),
            target: archive.package.clone(),
            inner: ExtractArchiveOperationInner {
                destination: destination.to_owned(),
                files: Vec::new(),
            },
        }
    }

    // TODO: use global factory
    pub fn new_from_weak_struct(package: Package, weak_struct: &str) -> Result<Self, rust_i18n::error::Error> {
        let inner: ExtractArchiveOperationInner = serializer::from_json(weak_struct)?;
        Ok(ExtractArchiveOperation { archive: None, target: package, inner })
    }
}

impl Serialize for ExtractArchiveOperation<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        self.inner.serialize(serializer)
    }
}

impl OperationPerformer for ExtractArchiveOperation<'_> {
    fn from_record(package: Option<Package>, record: &OperationRecord) -> Result<Self, rust_i18n::error::Error> {
        Self::new_from_weak_struct(package.expect("ExtractArchiveOperation requires target Package object to perform its operations"), record.get_data())
    }

    fn execute(&mut self, app: &InstallyApp) -> Result<(), rust_i18n::error::Error> {
        let product = app.get_product();
        let package_file = self.archive.unwrap();
        let progress_closure = app.create_progress_closure();   

        let files = {
            let mut archive = package_file.handle.lock();
            archiving::zip_read::extract_to(
                archive.as_file_mut(),
                &product.get_path_to_package(&self.target),
                &progress_closure,
                Some(&package_file.sha1)
            )
                .map_err(|err| ArchiveError::from(err))?
        };

        self.files = files;
        Ok(())
    }

    fn revert(&mut self, app: &InstallyApp) -> Result<(), rust_i18n::error::Error> {
        let product = app.get_product();

        self.files.iter().into_iter().for_each(|file| {
            if let Err(err) = std::fs::remove_file(file.clone()) {
                log::error!("Failed to delete {:?}. It's included inside {} package. Trace: {}", file.clone(), &self.target.display_name, err);
            } else {
                log::trace!("Deleted {:?} of {} package.", file.clone(), &self.target.display_name);
            }
        });

        Ok(())
    }

    fn finalize(&mut self, app: &InstallyApp) -> Result<(), rust_i18n::error::Error> {
        Ok(())
    }

    fn description(&self) -> String {
        t!("actions.extract-archive-operation")
    }
    
    fn get_kind(&self) -> definitions::operation::OperationKind {
        definitions::operation::OperationKind::ExtractArchiveOperation
    }
    
    fn as_weak_struct(&self) -> Result<String, serializer::SerializationError> {
        Ok(serializer::to_json(&self.inner)?)
    }
}

impl Deref for ExtractArchiveOperation<'_> {
    type Target = ExtractArchiveOperationInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl DerefMut for ExtractArchiveOperation<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}