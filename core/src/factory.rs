
use crate::{workloads::{installer::{InstallerOptions, Product, InstallerWrapper, InstallerWorkloadState}, uninstaller::{UninstallerOptions, UninstallerWrapper, UninstallerWorkloadState}, abstraction::{InstallyApp, WorkloadResult, Worker, Workload}, updater::{UpdaterWrapper, UpdaterWorkloadState, UpdaterOptions}}};

pub enum WorkloadType {
    Installer(InstallerOptions),
    Updater(UpdaterOptions),
    Uninstaller(UninstallerOptions)
}

pub struct Executor {
    pub handle: tokio::task::JoinHandle<WorkloadResult>,
    pub app: InstallyApp
}

pub fn run(product_meta: &Product, settings: WorkloadType) -> Executor {
    let app = InstallyApp::new(product_meta.clone());

    if let Ok(_) = tokio::runtime::Handle::try_current() {
        return Executor { handle: run_inner(app.clone(), settings), app };
    }
 
    panic!("No running tokio runtime found.")
}

pub fn run_inner(app: InstallyApp, settings: WorkloadType) -> tokio::task::JoinHandle<WorkloadResult> {
    let join = match settings {
        WorkloadType::Installer(r) => {
            println!("Spawning installer workload thread");
            installer(InstallerWrapper::new_with_opts(app.clone(), r))
        },
        WorkloadType::Updater(r) => {
            println!("Spawning updater workload thread");
            updater(UpdaterWrapper::new_with_opts(app.clone(), r))
        },
        WorkloadType::Uninstaller(r) => {
            println!("Spawning uninstaller workload thread");
            uninstaller(UninstallerWrapper::new_with_opts(app.clone(), r))
        },
    };

   join
}

fn installer(wrapper: InstallerWrapper) -> tokio::task::JoinHandle<WorkloadResult> {
    tokio::spawn(async move {
        println!("Running installer workload");

        let workload = wrapper.run().await;
    
        match workload {
            Ok(()) => {
                log::info!("Workload completed");
                println!("Workload completed");

                wrapper.set_result(WorkloadResult::Ok);
                wrapper.set_workload_state(InstallerWorkloadState::Done);
                WorkloadResult::Ok
            },
            Err(err) => {
                log::error!("\n{err:?}");
                println!("Workload failed. \n{err:?}");

                let result = WorkloadResult::Error(err.to_string());
                wrapper.set_result(result.clone());
                wrapper.set_workload_state(InstallerWorkloadState::Interrupted(err.to_string()));
                result
            }
        }
    })
}

fn updater(wrapper: UpdaterWrapper) -> tokio::task::JoinHandle<WorkloadResult> {
    tokio::spawn(async move {
        println!("Running updater workload");

        let workload_result = wrapper.run().await;
    
        match workload_result {
            Ok(()) => {
                log::info!("Workload completed");
                println!("Workload completed");
                
                wrapper.set_result(WorkloadResult::Ok);
                wrapper.set_workload_state(UpdaterWorkloadState::Done);
                WorkloadResult::Ok
            },
            Err(err) => {
                log::error!("\n{err:?}");
                println!("Workload failed. \n{err:?}");

                let result = WorkloadResult::Error(err.to_string());
                wrapper.set_result(result.clone());
                wrapper.set_workload_state(UpdaterWorkloadState::Interrupted(err.to_string()));
                result
            }
            
        }
    })
}

fn uninstaller(wrapper: UninstallerWrapper) -> tokio::task::JoinHandle<WorkloadResult> {
    tokio::spawn(async move {
        println!("Running uninstaller workload");

        let workload_result = wrapper.run().await;
    
        match workload_result {
            Ok(()) => {
                log::info!("Workload completed");
                println!("Workload completed");

                wrapper.set_result(WorkloadResult::Ok);
                wrapper.set_workload_state(UninstallerWorkloadState::Done);
                WorkloadResult::Ok
            },
            Err(err) => {
                log::error!("\n{err:?}");
                println!("Workload failed. \n{err:?}");

                let result = WorkloadResult::Error(err.to_string());
                wrapper.set_result(result.clone());
                wrapper.set_workload_state(UninstallerWorkloadState::Interrupted(err.to_string()));
                result
            }
        }
    })
}