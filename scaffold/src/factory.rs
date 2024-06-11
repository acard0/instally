
use eframe::egui;
use instally_core::{definitions::app::InstallyApp, factory::{Executor, WorkloadKind}, workloads::noop::NoopOptions};

pub fn run(app: InstallyApp, settings: WorkloadKind, do_spawn_ui: bool) -> Executor {
    let executor = instally_core::factory::run(
        app,
        settings
    );
    
    if do_spawn_ui {
        spawn_ui(executor.app.clone());
    }

    executor
}

pub fn failed(err: rust_i18n::error::Error) -> Executor {
    run(InstallyApp::default(), WorkloadKind::Error(NoopOptions::default(), err.get_details().clone()), true)
}

pub fn spawn_ui(ctx: InstallyApp) {

    // build native opts
    let options = eframe::NativeOptions {
        // Hide the OS-specific "chrome" around the window:
        decorated: false,
        // To have rounded corners we need transparency:
        transparent: true,
        min_window_size: Some(egui::vec2(450.0, 175.0)),
        initial_window_size: Some(egui::vec2(450.0, 175.0)),
        centered: true,
        ..Default::default()
    };

    let app_wrapper = crate::app::AppWrapper::new(ctx);
    let _ = eframe::run_native(
        "instally", // unused title
        options,
        Box::new(move |_cc| {
            Box::new(app_wrapper)
        }),
    );
}