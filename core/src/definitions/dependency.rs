use std::{process::Command, sync::Arc};

use parking_lot::Mutex;

use super::package::Package;



#[derive(Clone, Debug)]
pub struct PackageFile {
    pub handle: Arc<Mutex<tempfile::NamedTempFile>>,
    pub sha1: String,
    pub package: Package
}

#[derive(Debug)]
pub struct DependencyFile {
    pub handle: tempfile::NamedTempFile,
}

impl DependencyFile {
    pub fn new(file: tempfile::NamedTempFile) -> Self {
        DependencyFile { handle: file }
    }

    pub fn execute(self, arguments: Vec<String>, attached: bool) -> std::io::Result<()> {
        let (_, path) = self.handle.keep().unwrap();

        let mut cmd = Command::new(format!("{}", path.to_str().unwrap()));
        cmd.args(arguments);

        let handle = match cmd.spawn() {
            Ok(handle) => handle,
            Err(err) => {
                log::trace!("Command {:?} failed with error {:?}", cmd, err);
                return Err(err)
            }
        };

        if attached {
            match handle.wait_with_output() {
                Ok(output) => {
                    if !output.status.success() {
                        log::trace!("Command {:?} failed with error {:?}", cmd, output);
                    }
                }
                Err(err) => {
                    log::trace!("Command {:?} failed with error {:?}", cmd, err);
                    return Err(err)
                }
            }
        }

        std::fs::remove_file(path)
    }
}
