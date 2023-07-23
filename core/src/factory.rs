
use crate::{workloads::{installer::{InstallerOptions, InstallerWrapper, InstallerWorkloadState}, uninstaller::{UninstallerOptions, UninstallerWrapper, UninstallerWorkloadState}, abstraction::{InstallyApp, Workload, WorkloadResult}, updater::{UpdaterWrapper, UpdaterWorkloadState, UpdaterOptions}, definitions::Product}, extensions::future::FutureSyncExt};

pub enum WorkloadType {
    Installer(InstallerOptions),
    Updater(UpdaterOptions),
    Uninstaller(UninstallerOptions)
}

pub struct Executor {
    pub handle: tokio::task::JoinHandle<WorkloadResult>,
    pub app: InstallyApp
}

pub struct RuntimeExecutor {
    pub executor: Executor,
    pub runtime: tokio::runtime::Runtime
}

pub fn run_tokio(product_meta: &Product, settings: WorkloadType) -> RuntimeExecutor {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let executor = runtime.block_on(async move {
        run(product_meta, settings)
    });

    RuntimeExecutor { executor, runtime }
}

pub fn run(product_meta: &Product, settings: WorkloadType) -> Executor {
    let app = InstallyApp::build(&product_meta)
        .wait().unwrap();

    if let Ok(_) = tokio::runtime::Handle::try_current() {
        return Executor { handle: run_inner(&app, settings), app };
    }
 
    panic!("No running tokio runtime found.")
}

fn run_inner(app: &InstallyApp, settings: WorkloadType) -> tokio::task::JoinHandle<WorkloadResult> {
    let join = match settings {
        WorkloadType::Installer(r) => {
            log::info!("Spawning installer workload thread");
            installer(InstallerWrapper::new_with_opts(app.clone(), r))
        },
        WorkloadType::Updater(r) => {
            log::info!("Spawning updater workload thread");
            updater(UpdaterWrapper::new_with_opts(app.clone(), r))
        },
        WorkloadType::Uninstaller(r) => {
            log::info!("Spawning uninstaller workload thread");
            uninstaller(UninstallerWrapper::new_with_opts(app.clone(), r))
        },
    };

   join
}

fn installer(wrapper: InstallerWrapper) -> tokio::task::JoinHandle<WorkloadResult> {
    tokio::spawn(async move {
        log::info!("Running installer workload");

        let workload = wrapper.run().await;
    
        match workload {
            Ok(()) => {
                log::info!("Workload completed");

                wrapper.app.set_workload_state(InstallerWorkloadState::Done.to_string());
                wrapper.app.set_result(WorkloadResult::Ok);
                WorkloadResult::Ok
            },
            Err(err) => {
                log::error!("\n{err:?}");
                log::info!("Workload failed. \n{err:?}");

                let result = WorkloadResult::Error(err.get_details().to_owned());
                wrapper.app.set_workload_state(InstallerWorkloadState::Interrupted(err.get_details().to_owned()));
                wrapper.app.set_result(result.clone());
                result
            }
        }
    })
}

fn updater(wrapper: UpdaterWrapper) -> tokio::task::JoinHandle<WorkloadResult> {
    tokio::spawn(async move {
        log::info!("Running updater workload");

        let workload_result = wrapper.run().await;
    
        match workload_result {
            Ok(()) => {
                log::info!("Workload completed");
                
                wrapper.app.set_workload_state(UpdaterWorkloadState::Done);
                wrapper.app.set_result(WorkloadResult::Ok);
                WorkloadResult::Ok
            },
            Err(err) => {
                log::error!("\n{err:?}");
                log::info!("Workload failed. \n{err:?}");

                let result = WorkloadResult::Error(err.get_details().to_owned());
                wrapper.app.set_workload_state(UpdaterWorkloadState::Interrupted(err.get_details().to_owned()));
                wrapper.app.set_result(result.clone());
                result
            }
            
        }
    })
}

fn uninstaller(wrapper: UninstallerWrapper) -> tokio::task::JoinHandle<WorkloadResult> {
    tokio::spawn(async move {
        log::info!("Running uninstaller workload");

        let workload_result = wrapper.run().await;
    
        match workload_result {
            Ok(()) => {
                log::info!("Workload completed");

                wrapper.app.set_workload_state(UninstallerWorkloadState::Done);
                wrapper.app.set_result(WorkloadResult::Ok);
                WorkloadResult::Ok
            },
            Err(err) => {
                log::error!("\n{err:?}");
                log::info!("Workload failed. \n{err:?}");

                let result = WorkloadResult::Error(err.get_details().to_owned());
                wrapper.app.set_workload_state(UninstallerWorkloadState::Interrupted(err.get_details().to_owned()));
                wrapper.app.set_result(result.clone());
                result
            }
        }
    })
}