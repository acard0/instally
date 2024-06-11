use serde::{Deserialize, Serialize};

use crate::{helpers::serializer::SerializationError, workloads::operations::{archive::ExtractArchiveOperation, createappentry::CreateAppEntryOperation, createfile::CreateFileOperation, createmaintinancetool::CreateMaintenanceToolOperation, createsymlink::CreateSymlinkOperation}};

use super::{app::InstallyApp, package::Package};

/// Represents an operation that can be executed, reverted and can be stored in 'OperationHistory' in order to be reverted when needed.
pub struct Operation<'a> {
    performer: Box<dyn OperationPerformer + 'a>,

    /// existing if re-created from a record
    record: Option<&'a OperationRecord>
}

impl<'a> Operation<'a> {
    /// Creates new operation 
    pub fn from_performer(performer: Box<dyn OperationPerformer + 'a>) -> Self {
        Operation { performer, record: None }
    }

    /// Attemps to reconstruct an operation from its OperationRecord
    pub fn from_record(target: Option<&Package>, record: &'a OperationRecord) -> Result<Self, rust_i18n::error::Error> {
        match record.get_kind() {
            OperationKind::ExtractArchiveOperation => {
                Ok(Operation { record: Some(record), performer: Box::new(ExtractArchiveOperation::from_record(target.cloned(), record)?) })
            },
            OperationKind::CreateAppEntryOperation => {
                Ok(Operation { record: Some(record), performer: Box::new(CreateAppEntryOperation::from_record(target.cloned(), record)?) })
            },
            OperationKind::CreateFileOperation => {
                Ok(Operation { record: Some(record), performer: Box::new(CreateFileOperation::from_record(target.cloned(), record)?) })
            },
            OperationKind::CreateMaintenanceToolOperation => {
                Ok(Operation { record: Some(record), performer: Box::new(CreateMaintenanceToolOperation::from_record(target.cloned(), record)?) })
            },
            OperationKind::CreateSymlinkOperation => {
                Ok(Operation { record: Some(record), performer: Box::new(CreateSymlinkOperation::from_record(target.cloned(), record)?) })
            }
        }
    }

    /// Executes underlaying operation and adds record of it to the operation history
    /// if 'target' package is supplied, operation record will be added to target package's operation history, to the global operation history otherwise.
    /// 
    /// Note: This method does not persist changes over the installation summary to the disk.
    pub fn execute(&mut self, app: &InstallyApp, target: Option<&Package>) -> Result<(), rust_i18n::error::Error> {
        log::info!("Starting to execute '{:?}'.", self.get_kind());
        self.performer.execute(app)?;

        log::info!("Operation '{:?}' is executed, finalizing.", self.get_kind());

        self.performer.finalize(app)?;
        log::info!("Operation '{:?}' completed.", self.get_kind());

        app.modify_summary(|summary| {
            let history = match target {
                None => {
                    &mut summary.operations
                },
                Some(package) => {
                    &mut summary.find_mut(&package).expect(&format!("Package installation meta for '{}' is not found.", package.name)).operations
                }
            };

            history.add_from_operation(self).map(|sum| ())
        })?;

        Ok(())
    }

    /// Attemtps to revert underlaying operation
    pub fn revert(&mut self, app: &InstallyApp, target: Option<&Package>) -> Result<(), rust_i18n::error::Error> {
        log::info!("Starting to revert '{:?}'.", self.get_kind());
        self.performer.revert(app)?;

        log::info!("Operation '{:?}' is reverted, finalizing.", self.get_kind());

        self.performer.finalize(app)?;
        log::info!("Reverting '{:?}' completed.", self.get_kind());

        // record is present which means this operation record is loaded from disk, remove  it
        if self.record.is_some() {
            app.modify_summary(|summary| {
                let history = match target {
                    None => {
                        &mut summary.operations
                    },
                    Some(package) => {
                        &mut summary.find_mut(&package).expect(&format!("Package installation meta for '{}' is not found.", package.name)).operations
                    }
                };
    
                history.remove(self.record.as_ref().unwrap());
            });    
        }
        
        Ok(())
    }

    /// Gets description of the underlaying operation
    pub fn description(&self) -> String {
        self.performer.description()
    }

    /// Gets kind of the underlaying operation
    pub fn get_kind(&self) -> OperationKind {
        self.performer.get_kind()
    }

    /// Gets concrete operation as 'OperationRecord'. Seralization error might accur while serializing the fields of the underlaying operation
    pub fn as_record(&self) -> Result<OperationRecord, rust_i18n::error::Error> {
        self.performer.as_record()
    }
}

/// Interface for concrete operation types.
pub trait OperationPerformer {
    /// Attemps to reconstruct concerete operation from an OperationRecord
    fn from_record(package: Option<Package>, record: &OperationRecord) -> Result<Self, rust_i18n::error::Error> where Self: Sized;

    /// Attempts to execute concrete operation
    fn execute(&mut self, app: &InstallyApp) -> Result<(), rust_i18n::error::Error>;

    /// Attempts to finalize concrete operation
    fn finalize(&mut self, app: &InstallyApp) -> Result<(), rust_i18n::error::Error>;

    /// Attemtps to revert concrete operation
    fn revert(&mut self, app: &InstallyApp) -> Result<(), rust_i18n::error::Error>;

    /// Gets description of the concrete operation
    fn description(&self) -> String;

    /// Gets kind of the concrete operation
    fn get_kind(&self) -> OperationKind;

    /// Gets concrete operation as 'OperationRecord'. Seralization error might accur while serializing the fields of the concrete operation
    fn as_record(&self) -> Result<OperationRecord, rust_i18n::error::Error> {
        let payload = self.as_weak_struct()?;
        Ok(OperationRecord::new(self.get_kind(), payload))
    }

    /// Serializes concrete operation
    fn as_weak_struct(&self) -> Result<String, SerializationError>;
}

/// Represents operation history that can used to reconstruct individual operations in order to revert them.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct OperationHistory {
    records: Vec<OperationRecord>,
}

impl OperationHistory {
    /// Creates a new 'OperationHistory' from vector of operations.
    pub fn from_operations(operations: Vec<Operation>) -> Self {
        let records = operations.iter()
            .map(|op| op.as_record().unwrap())
            .collect::<Vec<_>>();

        OperationHistory { records }
    }

    /// Adds an operation record to the history using operation instance. 
    /// This operation can fail as serialization error might accur while serializing the concrete 'Operation' type.
    pub fn add_from_operation<'a>(&mut self, operation: &Operation<'a>) ->  Result<&Self, rust_i18n::error::Error>{
        self.records.push(operation.as_record()?);
        Ok(self)
    }

    /// Adds specified operation to the operation history
    pub fn add(&mut self, record: OperationRecord) -> &Self {
        self.records.push(record);
        self
    }

    /// Removed specified operation from the operation history
    pub fn remove(&mut self, record: &OperationRecord) -> &Self {
        if let Some(pos) = self.records.iter().position(|op| op == record) {
            self.records.remove(pos);
        }

        self
    }

    /// Gets operation records present in the history
    pub fn get_records(&self) -> &[OperationRecord] {
        &self.records
    }
}

/// Represents an entry of 'Operation' in 'OperationHistory'
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct OperationRecord {
    kind: OperationKind,
    data: String,
}

impl OperationRecord {
    /// Creates a new 'OperationRecord' from its details
    pub fn new(kind: OperationKind, data: String) -> Self {
        OperationRecord { kind, data }
    }

    /// Gets kind of the Operation this record represents
    pub fn get_kind(&self) -> &OperationKind {
        &self.kind
    }

    /// Gets serialized data of the Operation this record represents
    pub fn get_data(&self) -> &str {
        &self.data
    }

    /// Reconstructs the concrete Operation type this record represents
    pub fn into_operation(&self, package: Option<&Package>) -> Result<Operation, rust_i18n::error::Error> {
        Operation::from_record(package, &self)
    }
}

/// Available operation kinds
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum OperationKind {
    ExtractArchiveOperation,
    CreateFileOperation,
    CreateSymlinkOperation,
    CreateMaintenanceToolOperation,
    CreateAppEntryOperation
}

impl Default for OperationKind {
    fn default() -> Self {
        OperationKind::ExtractArchiveOperation
    }
}