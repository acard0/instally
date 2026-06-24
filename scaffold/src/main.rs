#![allow(dead_code, unused_variables)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use core::panic;
use std::{backtrace::Backtrace, thread, time::Duration};
use instally_core::{definitions::{app::InstallyApp, product::Product}, factory::WorkloadKind, helpers::serializer, workloads::{installer::InstallerOptions, uninstaller::UninstallerOptions, updater::UpdaterOptions}};

mod factory;
mod app;

use instally_core::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> { 
    let rust_log = std::env::var("RUST_LOG").unwrap_or("info".into()); 
    std::env::set_var("RUST_LOG", rust_log);  
    std::env::set_var("RUST_BACKTRACE", "1"); 
    _= env_logger::try_init();

    if std::env::args().any(|a| a == "--debug") {
        _= instally_core::sys::alloc_console();
        log::set_max_level(log::LevelFilter::Trace);
    }

    let locale = locale();

    println!("locale: {locale:?}");
    println!("translate: {}", t!("messages.hello"));

    std::panic::set_hook(Box::new(|info| {
        let payload = info.payload();
        let panic_msg = if let Some(s) = payload.downcast_ref::<&str>() {
            *s
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.as_str()
        } else {
            "Unknown panic payload"
        };

        let location = if let Some(loc) = info.location() {
            format!("{}:{}", loc.file(), loc.line())
        } else {
            "<unknown>".into()
        };

        let bt = Backtrace::capture();
        log::error!(
            "thread panicked with '{}' at [{}]\nBacktrace:\n{:?}",
            panic_msg, location, bt
        );

        thread::sleep(Duration::from_secs(5));
    }));

    let template_result: Result<Product, serializer::SerializationError> = serializer::from_json(PAYLOAD.strip_prefix("###/PAYLOAD/###").unwrap());
    let product = match template_result {
        Ok(template) => {
            log::info!("Payload meta for '{}' is valid. Using it.", &template.name);
            Product::from_template(template)
                .map_err(|err| format!("Failed to format product from template: {:?}", err))?
        },
        Err(_) => {
            #[cfg(not(debug_assertions))]
            {
                log::info!("Payload Product is not valid.");
                return Ok(());
            }

            log::info!("Payload Product is not valid and we are in debug env. Using dummy product for testing.");
            Product::from_template(
                Product::new(
                    "Tutucu Unity",
                    "@{App.Name}",
                    "liteware.io",
                    "https://liteware.io",
                    "https://cdn.liteware.xyz/downloads/tutucu/beta/",
                    "global_script.js",
                    "@{Directories.User.Home}\\AppData\\Local\\@{App.Publisher}\\@{App.Name}",
                )
            ).unwrap()
        }
    };

    let app = match InstallyApp::new(&product) {
        Ok(app) => app,
        Err(err) => {
            _ = factory::failed(&product, err.into());
            log::info!("Exit(0)");
            return Ok(());
        }
    };

    let args = parse_args();
    _ = factory::run(
        app,
        args.workload_type,
        !args.silent
    ).handle.await;

    log::info!("Exit(0)");
    Ok(())
}


struct Args {
    workload_type: WorkloadKind,
    silent: bool,
    debug: bool,
}

fn parse_args() -> Args {
    let mut args = std::env::args();
    let mut command = None;
    let mut silent = false;
    let mut debug = false;
    let mut target_packages: Option<Vec<String>> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            x if x.starts_with('/') => {
                if command.is_none() {
                    command = Some(x.to_string());
                    continue;
                }

                panic!("Multiple commands specified!");
            },
            "--silent" => silent = true,
            "--debug" => debug = true,
            "--packages" => {
                args.by_ref().take_while(|a| !a.starts_with('-')).for_each(|a| {
                    target_packages.get_or_insert_with(Vec::new).push(a);
                });
            },
            _ => { }
        }
    }

    let workload = match command.unwrap_or("/install".to_owned()).as_str() {
        "/install" => WorkloadKind::Installer(InstallerOptions::new(target_packages)),
        "/uninstall" => WorkloadKind::Uninstaller(UninstallerOptions::new(target_packages)),
        "/update" => WorkloadKind::Updater(UpdaterOptions::new(target_packages)),
        _ => panic!("Unrecognized command!")
    };

    Args {
        workload_type: workload,
        silent,
        debug
    }
}

// could not come with something more stupid
const PAYLOAD: &str = "###/PAYLOAD/###                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                ";
