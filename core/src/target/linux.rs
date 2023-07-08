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