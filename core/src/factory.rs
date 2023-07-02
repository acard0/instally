
use crate::workloads::{installer::{InstallerOptions, Product, InstallerWrapper, InstallerWorkloadState}, uninstaller::{UninstallerOptions, UninstallerWrapper, UninstallerWorkloadState}, abstraction::{InstallyApp, WorkloadResult, Worker, Workload}, updater::{UpdaterWrapper, UpdaterWorkloadState, UpdaterOptions}};

pub enum WorkloadType {
    Installer(InstallerOptions),
    Updater(UpdaterOptions),
    Uninstaller(UninstallerOptions)
}

pub struct Executor {
    pub handle: tokio::task::JoinHandle<WorkloadResult>,
    pub ctx: InstallyApp
}

pub fn run(product_meta: &Product, settings: WorkloadType) -> Executor {
    let ctx = InstallyApp::new(product_meta.clone());

    let join = match settings {
        WorkloadType::Installer(r) => {
            installer(InstallerWrapper::new_with_opts(ctx.clone(), r))
        },
        WorkloadType::Updater(r) => {
            updater(UpdaterWrapper::new_with_opts(ctx.clone(), r))
        },
        WorkloadType::Uninstaller(r) => {
            uninstaller(UninstallerWrapper::new_with_opts(ctx.clone(), r))
        },
    };

    Executor { handle: join, ctx }
}

pub fn installer(wrapper: InstallerWrapper) -> tokio::task::JoinHandle<WorkloadResult> {
    tokio::spawn(async move {
        let workload = wrapper.run().await;
    
        match workload {
            Ok(()) => {
                log::info!("Workload completed");
                wrapper.set_result(WorkloadResult::Ok);
                wrapper.set_workload_state(InstallerWorkloadState::Done);
                WorkloadResult::Ok
            },
            Err(err) => {
                log::error!("\n{err:?}");
                let result = WorkloadResult::Error(err.to_string());
                wrapper.set_result(result.clone());
                wrapper.set_workload_state(InstallerWorkloadState::Interrupted(err.to_string()));
                result
            }
        }
    })
}

pub fn updater(wrapper: UpdaterWrapper) -> tokio::task::JoinHandle<WorkloadResult> {
    tokio::spawn(async move {
        let workload_result = wrapper.run().await;
    
        match workload_result {
            Ok(()) => {
                println!("Workload completed");
                wrapper.set_result(WorkloadResult::Ok);
                wrapper.set_workload_state(UpdaterWorkloadState::Done);
                WorkloadResult::Ok
            },
            Err(err) => {
                log::error!("\n{err:?}");
                let result = WorkloadResult::Error(err.to_string());
                wrapper.set_result(result.clone());
                wrapper.set_workload_state(UpdaterWorkloadState::Interrupted(err.to_string()));
                result
            }
            
        }
    })
}

pub fn uninstaller(wrapper: UninstallerWrapper) -> tokio::task::JoinHandle<WorkloadResult> {
    tokio::spawn(async move {
        let workload_result = wrapper.run().await;
    
        match workload_result {
            Ok(()) => {
                println!("Workload completed");
                wrapper.set_result(WorkloadResult::Ok);
                wrapper.set_workload_state(UninstallerWorkloadState::Done);
                WorkloadResult::Ok
            },
            Err(err) => {
                log::error!("\n{err:?}");
                let result = WorkloadResult::Error(err.to_string());
                wrapper.set_result(result.clone());
                wrapper.set_workload_state(UninstallerWorkloadState::Interrupted(err.to_string()));
                result
            }
        }
    })
}