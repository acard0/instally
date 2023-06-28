#![allow(dead_code, unused_variables)]

//! Show a custom window frame instead of the default OS window chrome decorations.

// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod http;
mod app;
mod workloads;
mod archiving;
mod helpers;

use crate::{app::AppWrapper, workloads::abstraction::Worker};

use std::{sync::{Arc}};

use eframe::{egui};
use parking_lot::{Mutex};
use workloads::{installer::*, abstraction::*, uninstaller::*, updater::{UpdaterWorkloadState}};

pub type InstallerContext = AppContext<InstallerWorkloadState>;
pub type InstallerApp = InstallyApp<InstallerWorkloadState>;

pub type UninstallerContext = AppContext<UninstallerWorkloadState>;
pub type UninstallerApp = InstallyApp<UninstallerWorkloadState>;

pub type UpdaterContext = AppContext<UpdaterWorkloadState>;
pub type UpdaterApp = InstallyApp<UpdaterWorkloadState>;

pub type ContextArcT<T> = Arc<Mutex<AppContext<T>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rust_log = std::env::var("RUST_LOG").unwrap_or("info".into()); 
    std::env::set_var("RUST_LOG", rust_log);  
    env_logger::init();

    let payload_result = quick_xml::de::from_str(PAYLOAD.strip_prefix("###/PAYLOAD/###").unwrap());
    let product = match payload_result {
        Ok(r) => {
            log::info!("Payload Product is valid. Using it.");
            r
        },
        Err(_) => {
            log::info!("Payload Product is not valid. Using dummy.");

            //TODO: strip this from production
            Product { // prototype 'Product' structure
                name: "Tutucu".to_owned(),
                publisher: "liteware".to_owned(),
                product_url: "https://liteware.io".to_owned(),
                control_script: "none".to_owned(),
                target_directory: "C:\\Users\\doquk\\AppData\\Roaming\\liteware.io\\Tutucu".to_owned(),
                repository: "https://cdn.liteware.xyz/instally/tutucu/release/".to_owned()
            }
        }
    };

    log::info!("Payload xml: {:?}", quick_xml::se::to_string(&product));

    // create app state holder that is thread-safe
    let app = Box::leak(Box::new(|| {
        InstallerApp::new(product.clone())
    }))();

    // spawn worker thread
    let clone_worker = app.clone();
    tokio::spawn(async move {


/*
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
 */

 

    /*

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

    */
        

        
        let installer = InstallerWrapper::new(clone_worker); 
        let workload_result = installer.run().await;

        match workload_result {
            Ok(()) => {
                installer.set_result(WorkloadResult::Ok);
                installer.set_workload_state(InstallerWorkloadState::Done);
                println!("Workload completed");
            },
            Err(err) => {
                log::error!("\n{err:?}");
                installer.set_result(WorkloadResult::Error(err.to_string()));
                installer.set_workload_state(InstallerWorkloadState::Interrupted(err.to_string()));
            }
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


// could not come with something more stupid
const PAYLOAD: &str = "###/PAYLOAD/###<?xml version=\\\"1.0\\\" encoding=\\\"UTF-8\\\"?><Product><Name>Tutucu</Name><Publisher>liteware</Publisher><ProductUrl>https://liteware.io</ProductUrl><ControlScript>none</ControlScript><TargetDirectory>C:\\Users\\doquk\\AppData\\Roaming\\liteware.io\\Tutucu</TargetDirectory><Repository>https://cdn.liteware.xyz/instally/tutucu/release/</Repository></Product>                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                ";
