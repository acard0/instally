#![allow(dead_code, unused_variables)]

//! Show a custom window frame instead of the default OS window chrome decorations.

// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod http;
mod app;
mod workloads;

use crate::{app::AppWrapper};

use std::{sync::{Arc}};

use eframe::{egui};
use parking_lot::{Mutex};
use workloads::{installer::{InstallerWorkloadState, InstallerWrapper}, abstraction::{InstallyApp, Workload, AppContext}};

pub type InstallerContext = AppContext<InstallerWorkloadState>;
pub type InstallerApp = InstallyApp<InstallerWorkloadState>;

pub type ContextArcT<T> = Arc<Mutex<AppContext<T>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // create app state holder that is thread-safe
    let app = Box::leak(Box::new(|| {
        InstallerApp::default()
    }))();

    // spawn worker thread
    let clone_worker = app.clone();
    tokio::spawn(async move {
        let installer = InstallerWrapper::new(clone_worker); 
        let workload_result = installer.run().await;

        if !workload_result.is_ok() {
            let msg = workload_result.get_error().unwrap();
            log::error!("{msg}");
        }
    });

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

    // run the app
    let clone_ui = app.clone();
    let app_wrapper = AppWrapper::new(clone_ui);
    let _ = eframe::run_native(
        "instally", // unused title
        options,
        Box::new(move |_cc| {
            Box::new(app_wrapper)
        }),
    );

    Ok(())
}
