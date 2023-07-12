#![allow(dead_code, unused_variables)]

mod macros;
mod ffi;

use ffi::{CallResult, CPackageVersioning, CAppState};
use instally_core::{self, workloads::{installer::{Product, InstallitionSummary, Package, InstallerOptions, Repository}, updater::UpdaterOptions, abstraction::AppContextNotifiable, uninstaller::UninstallerOptions}, factory::{WorkloadType, self}, extensions::future::FutureSyncExt};

use crate::ffi::ByteBuffer;

pub struct Meta {
    pub product: Product,
    pub repository: Repository,
    pub installition_summary: InstallitionSummary
}

impl Meta {
    pub fn get() -> Self {
        let product = Product::read().unwrap();
        let repository = product.fetch_repository().wait().unwrap();
        let installition_summary = InstallitionSummary::read_or_create_target(&product).unwrap();

        Meta {
            product,
            repository,
            installition_summary
        }
    }
}

#[no_mangle] 
pub unsafe extern "C" fn check_updates(m_packages: *mut ByteBuffer) -> *mut CallResult::<ByteBuffer> {  
    let packages = m_packages.read()
        .into_string_vec();
     
    println!("Target packages are {:?}", packages);

    let meta = Meta::get();

    let packages = meta.repository.packages.iter()
        .filter(|f| packages.contains(&f.name))
        .cloned().collect::<Vec<Package>>();

    println!("Installed packages that are targetted: {:?}", packages);

    let version_summary = meta.installition_summary.cross_check(&packages).unwrap();
    let c_arr  = version_summary.updates.iter()
        .map(|n| CPackageVersioning::new(n))
        .collect::<Vec<_>>();

    println!("Summary: {:?}", c_arr);

    CallResult::new(ByteBuffer::from_vec_struct(c_arr), None)
        .into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn apply_updates(m_packages: *mut ByteBuffer, state_callback: extern "C" fn(CAppState)) {
    let binding = m_packages.read();
    let packages = binding
        .into_slice::<CPackageVersioning>();

    let meta = Meta::get();

    let target_packages = meta.installition_summary.packages.iter()
        .filter(|f| packages.iter().any(|p| p.get_name() == f.name))
        .cloned().collect::<Vec<_>>();

    execute_blocking(
        &meta.product,
        WorkloadType::Updater(UpdaterOptions { target_packages: Some(target_packages) }),
        state_callback
    );
}

#[no_mangle]
pub unsafe extern "C" fn remove_package(m_packages: *mut ByteBuffer, state_callback: extern "C" fn(CAppState)) {
    let binding = m_packages.read();
    let packages = binding
        .into_slice::<CPackageVersioning>();

    let meta = Meta::get();

    let target_packages = meta.installition_summary.packages.iter()
    .filter(|f| packages.iter().any(|p| p.get_name() == f.name))
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
     
    println!("Target packages are {:?}", packages);

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
    let runtime_executor = factory::run_tokio(&product_meta, settings);

    runtime_executor.executor.app.get_context().lock().subscribe(Box::new(move |f| {
        state_callback(f.state_cloned.clone().into());
        println!("change. state: {:?}, changed: {:?}, progress: {}", f.state_cloned.get_state(), f.field_cloned, f.state_cloned.get_progress());
    }));

    let out = runtime_executor.runtime
        .block_on(runtime_executor.executor.handle);

    println!("result?: {out:?}");
}