#![allow(dead_code, unused_variables)]

mod macros;
mod ffi;

use std::sync::atomic::AtomicBool;

use ffi::{CallResult, CPackageVersioning, CAppState};
use instally_core::{self, extensions::future::FutureSyncExt, factory::{self, WorkloadType}, workloads::{abstraction::AppContextNotifiable, definitions::{ByteBuffer, InstallitionSummary, Package, Product, Repository}, installer::InstallerOptions, uninstaller::UninstallerOptions, updater::UpdaterOptions}};

static ON_WORK: AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub struct Meta {
    pub product: Product,
    pub repository: Repository,
    pub installition_summary: InstallitionSummary
}

impl Meta {
    pub fn get() -> Self {
        let product = Product::read().unwrap();
        let repository = product.fetch_repository().wait().unwrap();
        let installition_summary = InstallitionSummary::read().unwrap();

        Meta {
            product,
            repository,
            installition_summary
        }
    }
}

#[no_mangle] 
pub unsafe extern "C" fn check_updates(m_packages: *mut ByteBuffer) -> *mut CallResult::<ByteBuffer> {  
    let meta = Meta::get();
    let packages = match m_packages {
        buff if (*buff).ptr() != std::ptr::null_mut() => {
            let packages = m_packages.read()
                .into_string_vec();

            log::info!("Target packages are: {:?}", packages);   

            meta.repository.packages.iter()
                .filter(|f| packages.contains(&f.name))
                .cloned().collect::<Vec<Package>>()
        },
        _ => {
            log::info!("Target packages are: all");
            meta.repository.packages.clone()
        }
    };

    let version_summary = meta.installition_summary.cross_check(&packages).unwrap();
    let mut c_arr  = version_summary.map.iter()
        .map(|n| CPackageVersioning::new(n))
        .collect::<Vec<_>>();

    version_summary.not_installed.iter().for_each(|n| {
        c_arr.push(CPackageVersioning::new_not_installed(n));
    });

    CallResult::new(ByteBuffer::from_vec_struct(c_arr), None)
        .into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn apply_updates(m_packages: *mut ByteBuffer, state_callback: extern "C" fn(CAppState)) {
    let packages = m_packages.read()
        .into_string_vec();
    log::info!("Target packages are: {:?}", packages);

    let meta = Meta::get();

    let target_packages = meta.installition_summary.packages.iter()
        .filter(|f| packages.iter().any(|p| p == &f.name))
        .cloned().collect::<Vec<_>>();

    execute_blocking(
        &meta.product,
        WorkloadType::Updater(UpdaterOptions { target_packages: Some(target_packages) }),
        state_callback
    );
}

#[no_mangle]
pub unsafe extern "C" fn remove_package(m_packages: *mut ByteBuffer, state_callback: extern "C" fn(CAppState)) {
    let packages = m_packages.read()
        .into_string_vec();

    log::info!("Target packages are: {:?}", packages);

    let meta = Meta::get();

    let target_packages = meta.installition_summary.packages.iter()
        .filter(|f| packages.iter().any(|p| p == &f.name))
        .cloned().collect::<Vec<_>>();

    execute_blocking(
        &meta.product,
        WorkloadType::Uninstaller(UninstallerOptions { target_packages: Some(target_packages) }),
        state_callback
    );
}

#[no_mangle]
pub unsafe extern "C" fn install_package(m_packages: *mut ByteBuffer, state_callback: extern "C" fn(CAppState)) {
    let packages = m_packages.read()
        .into_string_vec();
     
    log::info!("Target packages are {:?}", packages);

    let meta = Meta::get();

    let packages = meta.repository.packages.iter()
        .filter(|f| packages.contains(&f.name))
        .cloned().collect::<Vec<Package>>();

    execute_blocking(
        &meta.product,
        WorkloadType::Installer(InstallerOptions { target_packages: Some(packages) }),
        state_callback
    );
}

fn execute_blocking(product_meta: &Product, settings: WorkloadType, state_callback: extern "C" fn(CAppState)) {
    if ON_WORK.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }

    ON_WORK.store(true, std::sync::atomic::Ordering::Relaxed);

    let runtime_executor = factory::run_tokio(&product_meta, settings);

    let sub_id = runtime_executor.executor.app.get_context().lock().subscribe(Box::new(move |f| {
        state_callback(f.state_cloned.clone().into());
    }));

    let out = runtime_executor.runtime
        .block_on(runtime_executor.executor.handle);

    runtime_executor.executor.app.get_context().lock().unsubscribe(sub_id);

    ON_WORK.store(false, std::sync::atomic::Ordering::Relaxed);
}