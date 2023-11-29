#![allow(dead_code, unused_variables)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use core::panic;

use instally_core::{workloads::{uninstaller::UninstallerOptions, updater::UpdaterOptions, definitions::{Product, InstallitionSummary}, installer::InstallerOptions}, factory::WorkloadType, helpers::serializer};

mod factory;
mod app;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rust_log = std::env::var("RUST_LOG").unwrap_or("info".into()); 
    std::env::set_var("RUST_LOG", rust_log);  
    env_logger::init();

    let template_result = quick_xml::de::from_str(PAYLOAD.strip_prefix("###/PAYLOAD/###").unwrap());
    let product = match template_result {
        Ok(template) => {
            log::info!("Payload Product is valid. Using it.");
            Product::from_template(template)
                .map_err(|err| format!("Failed to create product from template: {}", err))?
        },
        Err(_) => {
            #[cfg(not(debug_assertions))]
            {
                log::info!("Payload Product is not valid.");
                return Ok(());
            }

            log::info!("Payload Product is not valid. Using dummy.");
            Product::from_template(
                Product::new(
                    "Wulite",
                    "@{App.Name}",
                    "liteware.io",
                    "https://liteware.io",
                    "@{Directories.User.Home}\\AppData\\Roaming\\@{App.Publisher}\\@{App.Name}",
                    "https://cdn.liteware.xyz/instally/wulite/",
                    "global_script.js",
                )
            ).unwrap()
        }
    };

    let cwd_eq = std::env::current_dir().unwrap() == product.get_target_directory();
    let summ_ok = InstallitionSummary::read().is_ok();
    
    if cwd_eq {
        // existing installition & installition summary is corrupted
        if !summ_ok {
            log::info!("Launched at target directory but installition summary not found/is invalid. Aborting.");
            return Ok(());
        } else {
            // set 'we are in maintinance mode'. installition seems valid.
            std::env::set_var("MAINTINANCE_EXECUTION", "1");  
            log::info!("At target directory & installition summary is present. Working as maintinance tool.");
        }
    // is this fresh installition or end-user moved installition folder?
    } else {
        // different target folder, ok...
        if summ_ok {
            // set 'we are in maintinance mode'. installition seems valid.
            std::env::set_var("MAINTINANCE_EXECUTION", "1");  
            log::warn!("Installition folder is moved & installition summary is present. Working as maintinance tool.");
        } else {
            // set 'we are in fresh installition mode'
            std::env::set_var("STANDALONE_EXECUTION", "1");
            log::info!("Fresh installition. Working as installer.");
        }
    }

    log::info!("Payload xml: {:?}", serializer::to_xml(&product));
    log::info!("Target directory: {:?}", &product.get_relative_target_directory());

    log::info!("Terminating processes under the target directory");
    instally_core::helpers::process::terminate_processes_under_folder(&product.get_relative_target_directory())
        .expect("Failed to terminate processes under the target directory!");

    let args = parse_args(&product).await;
    _ = factory::run(
        &product,
        args.workload_type,
        !args.silent

    ).handle.await;

    log::info!("Exit(0)");
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

    let installition_summary = InstallitionSummary::read().ok();

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

                    if let Some(summary) = &installition_summary {
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
const PAYLOAD: &str = "###/PAYLOAD/###                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                ";
