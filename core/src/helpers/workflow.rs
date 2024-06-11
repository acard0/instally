use crate::{definitions::{product::Product, summary::InstallationSummary}, helpers};

use super::file::IoError;

#[derive(Clone, Debug, PartialEq)]
pub enum Workflow {
    FreshInstallition,
    MaintenanceTool,
    FfiApi,
}

/// Defines workflow from current environment
pub fn define_workflow_env(product: &Product) -> Result<Workflow, IoError> {
    let cwd = helpers::file::cwd()?;
    std::env::set_var("CANONICALIZED_CWD", cwd.to_str().unwrap());
    log::info!("cwd: {:?}, target: {:?}", cwd, product.get_target_directory());

    // cwd can differ due to env. eg: when ran by windows through Programs and Features control panel cwd will be equal to system32 folder
    _ = std::env::set_current_dir(&cwd);

    let cwd_eq = cwd == product.get_target_directory();
    let summ_ok = InstallationSummary::read();

    if cwd_eq {
        // existing installition & installition summary is corrupted
        if summ_ok.is_err() {
            panic!("Launched at target directory but installition summary not found/is invalid. Aborting. {:?}", summ_ok.err().unwrap());
        } else {
            // set 'we are in maintinance mode'. installition seems valid.
            std::env::set_var("MAINTENANCE_EXECUTION", "1");  
            log::info!("At target directory & installition summary is present. Working as maintinance tool.");
        }
    // is this fresh installition or end-user moved installition folder?
    } else {
        // different target folder, ok...
        if summ_ok.is_ok() {
            // set 'we are in maintinance mode'. installition seems valid.
            std::env::set_var("MAINTENANCE_EXECUTION", "1");  
            log::warn!("Installition folder is moved & installition summary is present. Working as maintinance tool.");
        // summary not present, cwd is different. has to be fresh installation
        } else {
            // set 'we are in fresh installition mode'
            std::env::set_var("STANDALONE_EXECUTION", "1");
            log::info!("Fresh installition. Working as installer.");
        }
    }

    log::info!("Workflow env defined as {:?}", get_workflow_from_env());

    Ok(get_workflow_from_env())
}

/// Gets workflow from current environment
pub fn get_workflow_from_env() -> Workflow {
    if std::env::var("STANDALONE_EXECUTION").is_ok() {
        return Workflow::FreshInstallition;
    }

    if std::env::var("MAINTENANCE_EXECUTION").is_ok() {
        return Workflow::MaintenanceTool;
    }

    return Workflow::FfiApi;
}