use std::fmt::Display;

use instally_core::workloads::{installer::{Product, InstallerWrapper, InstallerWorkloadState}, abstraction::{InstallerApp, WorkloadResult, Worker, Workload, UpdaterApp, UninstallerApp, InstallyApp}, updater::{UpdaterAppWrapper, UpdaterWorkloadState}, uninstaller::{UninstallerWrapper, UninstallerWorkloadState}};

use crate::app::AppWrapper;

pub async fn installer(product: &Product, do_spawn_ui: bool) {

    // create app state holder that is thread-safe
    let app = Box::leak(Box::new(|| {
        InstallerApp::new(product.clone())
    }))();

    // spawn worker thread
    let clone_worker = app.clone();
    tokio::spawn(async move {
        let installer = InstallerWrapper::new(clone_worker); 
        let workload_result = installer.run().await;
    
        match workload_result {
            Ok(()) => {
                installer.set_result(WorkloadResult::Ok);
                installer.set_workload_state(InstallerWorkloadState::Done);
                log::info!("Workload completed");
            },
            Err(err) => {
                log::error!("\n{err:?}");
                installer.set_result(WorkloadResult::Error(err.to_string()));
                installer.set_workload_state(InstallerWorkloadState::Interrupted(err.to_string()));
            }
        }
    });

    if do_spawn_ui {
        spawn_ui(app.clone());
    }
}

pub async fn updater(product: &Product, do_spawn_ui: bool) {

    // create app state holder that is thread-safe
    let app = Box::leak(Box::new(|| {
        UpdaterApp::new(product.clone())
    }))();

    // spawn worker thread
    let clone_worker = app.clone();
    tokio::spawn(async move {
        let updater = UpdaterAppWrapper::new(clone_worker); 
        let workload_result = updater.run().await;
    
        match workload_result {
            Ok(()) => {
                updater.set_result(WorkloadResult::Ok);
                updater.set_workload_state(UpdaterWorkloadState::Done);
                println!("Workload completed");
            },
            Err(err) => {
                log::error!("\n{err:?}");
                updater.set_result(WorkloadResult::Error(err.to_string()));
                updater.set_workload_state(UpdaterWorkloadState::Interrupted(err.to_string()));
            }
            
        }
    });

    if do_spawn_ui {
        spawn_ui(app.clone());
    }
}

pub async fn uninstaller(product: &Product, do_spawn_ui: bool) {

    // create app state holder that is thread-safe
    let app = Box::leak(Box::new(|| {
        UninstallerApp::new(product.clone())
    }))();

    // spawn worker thread
    let clone_worker = app.clone();
    tokio::spawn(async move {
        let uninstaller = UninstallerWrapper::new(clone_worker); 
        let workload_result = uninstaller.run().await;
    
        match workload_result {
            Ok(()) => {
                uninstaller.set_result(WorkloadResult::Ok);
                uninstaller.set_workload_state(UninstallerWorkloadState::Done);
                println!("Workload completed");
            },
            Err(err) => {
                log::error!("\n{err:?}");
                uninstaller.set_result(WorkloadResult::Error(err.to_string()));
                uninstaller.set_workload_state(UninstallerWorkloadState::Interrupted(err.to_string()));
            }
        }
    });

    if do_spawn_ui {
        spawn_ui(app.clone());
    }
}

pub fn spawn_ui<TState>(app: InstallyApp<TState>) 
where TState: Display + Send + Clone + 'static {

    // build native opts
    let options = eframe::NativeOptions {
        // Hide the OS-specific "chrome" around the window:
        decorated: false,
        // To have rounded corners we need transparency:
        transparent: true,
        min_window_size: Some(egui::vec2(450.0, 150.0)),
        initial_window_size: Some(egui::vec2(450.0, 150.0)),
        ..Default::default()
    };

    let app_wrapper = AppWrapper::new(app);
    let _ = eframe::run_native(
        "instally", // unused title
        options,
        Box::new(move |_cc| {
            Box::new(app_wrapper)
        }),
    );
}