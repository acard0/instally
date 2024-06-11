
use rust_i18n::error::ErrorDetails;

use crate::{definitions::app::InstallyApp, extensions::future::FutureSyncExt, workloads::{installer::{InstallerOptions, InstallerWorkloadState, InstallerWrapper}, noop::{NoopOptions, NoopWorkloadState, NoopWrapper}, uninstaller::{UninstallerOptions, UninstallerWorkloadState, UninstallerWrapper}, updater::{UpdaterOptions, UpdaterWorkloadState, UpdaterWrapper}, workload::{Workload, WorkloadResult}}};

pub enum WorkloadKind {
    Installer(InstallerOptions),
    Updater(UpdaterOptions),
    Uninstaller(UninstallerOptions),
    Error(NoopOptions, ErrorDetails),
}

pub struct Executor {
    pub handle: tokio::task::JoinHandle<WorkloadResult>,
    pub app: InstallyApp
}

pub struct RuntimeExecutor {
    pub executor: Executor,
    pub runtime: tokio::runtime::Runtime
}

pub fn run_tokio(app: InstallyApp, settings: WorkloadKind) -> RuntimeExecutor {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let executor = runtime.block_on(async move {
        run(app, settings)
    });

    RuntimeExecutor { executor, runtime }
}

pub fn run(app: InstallyApp, settings: WorkloadKind) -> Executor {
    if let Ok(_) = tokio::runtime::Handle::try_current() {
        return Executor { handle: run_inner(&app, settings), app };
    }
 
    panic!("No running tokio runtime found.")
}

fn run_inner(app: &InstallyApp, settings: WorkloadKind) -> tokio::task::JoinHandle<WorkloadResult> {
    let join = match settings {
        WorkloadKind::Installer(r) => {
            log::info!("Spawning installer workload thread");
            installer(InstallerWrapper::new_with_opts(app.clone(), r))
        },
        WorkloadKind::Updater(r) => {
            log::info!("Spawning updater workload thread");
            updater(UpdaterWrapper::new_with_opts(app.clone(), r))
        },
        WorkloadKind::Uninstaller(r) => {
            log::info!("Spawning uninstaller workload thread");
            uninstaller(UninstallerWrapper::new_with_opts(app.clone(), r))
        },
        WorkloadKind::Error(opt, err) => {
            noop(err, NoopWrapper::new_with_opts(app.clone(), opt))
        }
    };

   join
}

fn installer(mut wrapper: InstallerWrapper) -> tokio::task::JoinHandle<WorkloadResult> {
    tokio::spawn(async move {
        log::info!("Running installer workload");
        let workload_result = wrapper.run().await;

        log::info!("Finalizing installer workload");
        let finalize_result = wrapper.finalize(workload_result.is_err()).wait(); // TODO: impl send + sync for err type

        match (workload_result, finalize_result) {
            (Ok(()), Ok(())) => {
                log::info!("Workload completed");
                
                let result = WorkloadResult::Ok;
                wrapper.app.set_workload_state(InstallerWorkloadState::Done);
                wrapper.app.set_result(&result);
                result
            },
            (Err(err), _) | (_, Err(err)) => {
                log::error!("\n{err:?}");
                log::info!("Workload failed. \n{err:?}");

                let result = WorkloadResult::Error(err.get_details().to_owned());
                wrapper.app.set_workload_state(InstallerWorkloadState::Interrupted(err.get_details().to_owned()));
                wrapper.app.set_result(&result);
                result
            },
        }
    })
}


fn updater(mut wrapper: UpdaterWrapper) -> tokio::task::JoinHandle<WorkloadResult> {
    tokio::spawn(async move {
        log::info!("Running updater workload");
        let workload_result = wrapper.run().await;

        log::info!("Finalizing updater workload");
        let finalize_result = wrapper.finalize(workload_result.is_err()).wait(); // TODO: impl send + sync for err type
    
        match (workload_result, finalize_result) {
            (Ok(()), Ok(())) => {
                log::info!("Workload completed");
                
                let result = WorkloadResult::Ok;
                wrapper.app.set_workload_state(UpdaterWorkloadState::Done);
                wrapper.app.set_result(&result);
                result
            },
            (Err(err), _) | (_, Err(err)) => {
                log::error!("\n{err:?}");
                log::info!("Workload failed. \n{err:?}");

                let result = WorkloadResult::Error(err.get_details().to_owned());
                wrapper.app.set_workload_state(UpdaterWorkloadState::Interrupted(err.get_details().to_owned()));
                wrapper.app.set_result(&result);
                result
            },
        }
    })
}

fn uninstaller(mut wrapper: UninstallerWrapper) -> tokio::task::JoinHandle<WorkloadResult> {
    tokio::spawn(async move {
        log::info!("Running uninstaller workload");
        let workload_result = wrapper.run().await;

        log::info!("Finalizing uninstaller workload");
        let finalize_result = wrapper.finalize(workload_result.is_err()).wait(); // TODO: impl send + sync for err type

        match (workload_result, finalize_result) {
            (Ok(()), Ok(())) => {
                log::info!("Workload completed");
                
                let result = WorkloadResult::Ok;
                wrapper.app.set_workload_state(UninstallerWorkloadState::Done);
                wrapper.app.set_result(&result);
                result
            },
            (Err(err), _) | (_, Err(err)) => {
                log::error!("\n{err:?}");
                log::info!("Workload failed. \n{err:?}");

                let result = WorkloadResult::Error(err.get_details().to_owned());
                wrapper.app.set_workload_state(UninstallerWorkloadState::Interrupted(err.get_details().to_owned()));
                wrapper.app.set_result(&result);
                result
            },
        }
    })
}

fn noop(err: ErrorDetails, mut wrapper: NoopWrapper) -> tokio::task::JoinHandle<WorkloadResult> {
    tokio::spawn(async move { 
        log::info!("Could not initiate a workload.");

        let workload_result = wrapper.run().await;
        let finalize_result = wrapper.finalize(workload_result.is_err()).wait(); // TODO: impl send + sync for err type

        let result = WorkloadResult::Error(err);
        wrapper.app.set_workload_state(NoopWorkloadState::Done);
        wrapper.app.set_result(&result);
        result
    })
}