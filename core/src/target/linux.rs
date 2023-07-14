use std::path::Path;

use crate::AppInstallition;
use crate::error::CreateAppEntryError;
use crate::error::CreateSymlinkError;
use crate::like::CStringLike;

pub fn symlink_file<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link_dir: Q, link_name: &str) -> Result<(), CreateSymlinkError> {
    unimplemented!() 
}

pub fn create_app_entry(app: AppInstallition) -> Result<(), CreateAppEntryError> {
    unimplemented!() 
}

pub fn break_symlink_file<P: AsRef<Path>>(link_dir: P, link_name: &str) -> std::io::Result<()> {
    unimplemented!() 
}


impl GlobalConfigImpl for GlobalConfig {
    fn new() -> Self {
        Self {  }
    }

    fn set(&self, key: String, name: String, value: String) -> Result<(), OsError> {
        unimplemented!()
    }

    fn get(&self, key: String, name: String) -> Result<String, OsError> {
        unimplemented!()
    }

    fn delete(&self, key: String) -> Result<(), OsError> {
        unimplemented!()
    }
}