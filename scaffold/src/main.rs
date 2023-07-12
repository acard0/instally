#![allow(dead_code, unused_variables)]
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use core::panic;

use instally_core::{workloads::{installer::{Product, InstallerOptions, InstallitionSummary}, uninstaller::UninstallerOptions, updater::UpdaterOptions}, factory::WorkloadType};

mod factory;
mod app;

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

    instally_core::helpers::process::terminate_processes_under_folder(&product.target_directory)
        .expect("Failed to terminate processes under target directory!");

    let args = parse_args(&product).await;

    _ = factory::run(
        &product,
        args.workload_type,
        !args.silent
    ).handle.await;

    Ok(())
}


struct Args {
    workload_type: WorkloadType,
    silent: bool,
}

async fn parse_args(product: &Product) -> Args {
    let repository = product
        .fetch_repository().await
        .unwrap();

    let installition_summary = InstallitionSummary::read_or_create_target(product)
        .ok();

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
                    let remote = repository.get_package(&a)
                        .expect(format!("Package {} not found!", &a).as_str());

                    target_packages.get_or_insert_with(|| Vec::new())
                        .push(remote.clone());

                    if let Some(summary) = installition_summary.as_ref() {
                        if let Some(local) = summary.find(&remote) {
                            target_local_packages.get_or_insert_with(|| Vec::new())
                                .push(local);
                        }
                    }
                });
            },
            _ => { }
        }
    }
    
    let workload = match command.unwrap_or("/install".to_owned()).as_str() {
        "/install" => WorkloadType::Installer(InstallerOptions {
            target_packages: target_packages
        }),
        "/uninstall" => WorkloadType::Uninstaller(UninstallerOptions {
            target_packages: target_local_packages
        }),
        "/update" => WorkloadType::Updater(UpdaterOptions {
            target_packages: target_local_packages
        }),
        _ => panic!("Unrecognized command!")
    };

    Args {
        workload_type: workload,
        silent,
    }
}

// could not come with something more stupid
const PAYLOAD: &str = "###/PAYLOAD/###<?xml version=\\\"1.0\\\" encoding=\\\"UTF-8\\\"?><Product><Name>Tutucu</Name><Publisher>liteware</Publisher><ProductUrl>https://liteware.io</ProductUrl><ControlScript>none</ControlScript><TargetDirectory>C:\\Users\\doquk\\AppData\\Roaming\\liteware.io\\Tutucu</TargetDirectory><Repository>https://cdn.liteware.xyz/instally/tutucu/release/</Repository></Product>                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                ";
