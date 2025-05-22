#![allow(dead_code, unused_variables)]

mod macros;
mod ffi;

use std::sync::atomic::AtomicBool;

use ffi::{CallResult, CPackageVersioning, CAppState};
use instally_core::{definitions::{app::InstallyApp, bytebuffer::ByteBuffer, context::AppContextNotifiable, package::Package, product::Product, repository::Repository}, extensions::future::FutureSyncExt, factory::{self, WorkloadKind}, workloads::{installer::InstallerOptions, uninstaller::UninstallerOptions, updater::UpdaterOptions, workload::WorkloadResult}};
static ON_WORK: AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub struct Meta {
    pub app: InstallyApp,
    pub repository: Repository,
}

impl Meta {
    pub fn get() -> Self {
        let product = Product::read().unwrap();
        let app = InstallyApp::build(&product).wait().unwrap();
        let repository = app.get_repository().clone();

        Meta {
            app,
            repository,
        }
    }
}

#[no_mangle] 
pub unsafe extern "C" fn init(m_packages: *mut ByteBuffer) {
    let rust_log = std::env::var("RUST_LOG").unwrap_or("info".into()); 
    std::env::set_var("RUST_LOG", rust_log);  
    std::env::set_var("RUST_BACKTRACE", "1"); 
    _= env_logger::try_init();
}

#[no_mangle] 
pub unsafe extern "C" fn check_updates(m_packages: *mut ByteBuffer) -> *mut CallResult::<ByteBuffer> {  
    let meta = Meta::get();
    let packages = match m_packages {
        buff if buff.is_null() == false && (*buff).len() > 0 => {
            let packages = m_packages.read()
                .into_string_vec();

            log::info!("Checking updates, target package(s): {:?}", packages);   

            meta.repository.packages.iter()
                .filter(|f| packages.contains(&f.name))
                .cloned().collect::<Vec<Package>>()
        },
        _ => {
            log::info!("Checking updates, target packages: all");
            meta.repository.packages.clone()
        }
    };

    let version_summary = meta.app.get_summary().cross_check(&packages);
    let mut c_arr  = version_summary.map.iter()
        .map(|n| CPackageVersioning::new(n))
        .collect::<Vec<_>>();

    version_summary.not_installed.iter().for_each(|n| {
        c_arr.push(CPackageVersioning::new_not_installed(n));
    });

    log::info!("Update check comlete, {}", version_summary);
    CallResult::new(ByteBuffer::from_vec_struct(c_arr), None).into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn apply_updates(m_packages: *mut ByteBuffer, state_callback: extern "C" fn(CAppState)) {
    let packages = m_packages.read().into_string_vec();
    log::info!("Appliying update(s), target package(s) are: {:?}", packages);

    let meta = Meta::get();
    let target_packages = meta.app.get_summary().packages.iter()
        .filter(|f| packages.iter().any(|p| p == &f.name))
        .cloned().collect::<Vec<_>>();

    let result = execute_blocking(
        &meta.app.get_product(),
        WorkloadKind::Updater(UpdaterOptions { target_packages: Some(target_packages) }),
        state_callback
    );

    if let Some(result) = result {
        log::info!("Update package operation complete, {}", result);
    }
}

#[no_mangle]
pub unsafe extern "C" fn remove_package(m_packages: *mut ByteBuffer, state_callback: extern "C" fn(CAppState)) {
    let packages = m_packages.read().into_string_vec();

    log::info!("Removing package(s), target package(s): {:?}", packages);

    let meta = Meta::get();

    let target_packages = meta.app.get_summary().packages.iter()
        .filter(|f| packages.iter().any(|p| p == &f.name))
        .cloned().collect::<Vec<_>>();

    let result = execute_blocking(
        &meta.app.get_product(),
        WorkloadKind::Uninstaller(UninstallerOptions { target_packages: Some(target_packages) }),
        state_callback
    );

    if let Some(result) = result {
        log::info!("Remove package operation complete, {}", result);
    }
}

#[no_mangle]
pub unsafe extern "C" fn install_package(m_packages: *mut ByteBuffer, state_callback: extern "C" fn(CAppState)) {
    let packages = m_packages.read().into_string_vec();
    
    log::info!("Target packages are {:?}", packages);

    let meta = Meta::get();

    let packages = meta.repository.packages.iter()
        .filter(|f| packages.contains(&f.name))
        .cloned().collect::<Vec<Package>>();

    let result = execute_blocking(
        &meta.app.get_product(),
        WorkloadKind::Installer(InstallerOptions { target_packages: Some(packages) }),
        state_callback
    );

    
    if let Some(result) = result {
        log::info!("Install package operation complete, {}", result);
    }
}

fn execute_blocking(product_meta: &Product, settings: WorkloadKind, state_callback: extern "C" fn(CAppState)) -> Option<WorkloadResult> {
    if ON_WORK.load(std::sync::atomic::Ordering::Relaxed) {
        return None;
    }

    ON_WORK.store(true, std::sync::atomic::Ordering::Relaxed);

    let meta = Meta::get();
    let executor = factory::run(meta.app.clone(), settings, None);

    let sub_id = executor.app.get_context().lock().subscribe(Box::new(move |f| {
        state_callback(f.state_cloned.clone().into());
    }));

    let result = executor.runtime.block_on(executor.handle).unwrap();
    
    executor.app.get_context().lock().unsubscribe(sub_id);
    ON_WORK.store(false, std::sync::atomic::Ordering::Relaxed);

    Some(result)
}