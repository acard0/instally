#![allow(dead_code, unused_variables)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use core::panic;

use instally_core::{definitions::{app::InstallyApp, product::Product}, factory::WorkloadKind, helpers::{self, serializer}, workloads::{installer::InstallerOptions, uninstaller::UninstallerOptions, updater::UpdaterOptions}};

mod factory;
mod app;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::panic::set_hook(Box::new(move |err| {
        helpers::file::write_all(".crash-report.log", format!("{:?}", err).as_bytes()).unwrap();
    }));
    
    let rust_log = std::env::var("RUST_LOG").unwrap_or("info".into()); 
    std::env::set_var("RUST_LOG", rust_log);  
    env_logger::init();

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
                    "Wulite Beta",
                    "@{App.Name}",
                    "liteware.io",
                    "https://liteware.io",
                    "https://cdn.liteware.xyz/downloads/wulite/beta/",
                    "global_script.js",
                    "@{Directories.User.Home}\\AppData\\Local\\@{App.Publisher}\\@{App.Name}",
                )
            ).unwrap()
        }
    };

    let result = InstallyApp::build(&product).await;    

    if result.is_err() {
        _ = factory::failed(result.err().unwrap().into())
    } else {
        let app = result.unwrap();
        let args = parse_args(&app).await;
        _ = factory::run(
            app,
            args.workload_type,
            !args.silent
        ).handle.await;
    }

    log::info!("Exit(0)");
    Ok(())
}


struct Args {
    workload_type: WorkloadKind,
    silent: bool,
}

async fn parse_args(app: &InstallyApp) -> Args {
    let mut args = std::env::args();
    let mut command = None;
    let mut silent = false;
    let mut target_packages = None;
    let mut target_local_packages = None;
    
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
            "--packages" => {
                args.by_ref().take_while(|a| !a.starts_with('-')).for_each(|a| {
                    let remote = app.get_repository().get_package(&a)
                        .expect(format!("Package {} not found!", &a).as_str());

                    target_packages.get_or_insert_with(|| Vec::new())
                        .push(remote.clone());

                    let summary = app.get_summary();
                    if let Some(local) = summary.find(&remote).cloned() {
                        target_local_packages.get_or_insert_with(|| Vec::new())
                            .push(local);
                    }
                });
            },
            _ => { }
        }
    }
    
    let workload = match command.unwrap_or("/install".to_owned()).as_str() {
        "/install" => WorkloadKind::Installer(InstallerOptions::new(target_packages)),
        "/uninstall" => WorkloadKind::Uninstaller(UninstallerOptions::new(target_local_packages)),
        "/update" => WorkloadKind::Updater(UpdaterOptions::new(target_local_packages)),
        _ => panic!("Unrecognized command!")
    };

    Args {
        workload_type: workload,
        silent,
    }
}

// could not come with something more stupid
const PAYLOAD: &str = "###/PAYLOAD/###                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                ";
