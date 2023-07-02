
use instally_core::{factory::{WorkloadType, Executor}, workloads::{abstraction::InstallyApp, installer::Product}};

pub fn run(product_meta: &Product, settings: WorkloadType, do_spawn_ui: bool) -> Executor {
    let executor = instally_core::factory::run(product_meta, settings);
    
    if do_spawn_ui {
        spawn_ui(executor.ctx.clone());
    }

    executor
}

pub fn spawn_ui(ctx: InstallyApp) {

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

    let app_wrapper = crate::app::AppWrapper::new(ctx);
    let _ = eframe::run_native(
        "instally", // unused title
        options,
        Box::new(move |_cc| {
            Box::new(app_wrapper)
        }),
    );
}