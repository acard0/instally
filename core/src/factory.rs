
use std::{sync::{atomic::{AtomicBool, Ordering}, Arc}, thread, time::Duration};

use rust_i18n::error::ErrorDetails;

use crate::{definitions::{app::InstallyApp, context::AppContextNotifiable}, extensions::future::FutureSyncExt, workloads::{installer::{InstallerOptions, InstallerWorkloadState, InstallerWrapper}, noop::{NoopOptions, NoopWorkloadState, NoopWrapper}, uninstaller::{UninstallerOptions, UninstallerWorkloadState, UninstallerWrapper}, updater::{UpdaterOptions, UpdaterWorkloadState, UpdaterWrapper}, workload::{Workload, WorkloadResult}}};

pub enum WorkloadKind {
    Installer(InstallerOptions),
    Updater(UpdaterOptions),
    Uninstaller(UninstallerOptions),
    Error(NoopOptions, ErrorDetails),
}

pub struct Executor {
    pub runtime: tokio::runtime::Handle,
    pub handle: tokio::task::JoinHandle<WorkloadResult>,
    pub app: InstallyApp
}

pub fn run(app: InstallyApp, settings: WorkloadKind, runtime: Option<&tokio::runtime::Runtime>) -> Executor {
    if let Some(rt) = runtime {
        return run_inner(app, settings, rt.handle().to_owned());
    }

    if let Ok(existing) = tokio::runtime::Handle::try_current() {
        return run_inner(app, settings, existing.to_owned());
    }

    let next = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let m_rt = Box::into_raw(Box::new(next));

    let mu_rt = m_rt as usize;
    let spawned = Arc::new(AtomicBool::new(false));
    let spawned_cloned = spawned.clone(); 
    app.get_context().lock().subscribe(Box::new(move |f| {
        if f.state_cloned.is_complete() && !spawned_cloned.swap(true, Ordering::SeqCst) {
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(10));
                _ = unsafe { Box::from_raw(mu_rt as *mut tokio::runtime::Runtime) };
            });
        }
    }));

    run(app, settings, Some(unsafe { &*m_rt }))
}

pub fn run_inner(app: InstallyApp, settings: WorkloadKind, runtime: tokio::runtime::Handle) -> Executor {
    let rt0 = runtime.clone();
    let executor = runtime.spawn_blocking(|| {
        Executor { runtime: rt0, handle: spawn_workload(&app, settings), app }
    });

    executor.wait().unwrap()
}

fn spawn_workload(app: &InstallyApp, settings: WorkloadKind) -> tokio::task::JoinHandle<WorkloadResult> {
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
                log::error!("Workload failed. \n{err:?}");

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
                log::error!("Workload failed. \n{err:?}");

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
                log::error!("Workload failed. \n{err:?}");

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