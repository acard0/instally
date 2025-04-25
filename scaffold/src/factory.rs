
use instally_core::{definitions::app::InstallyApp, factory::{Executor, WorkloadKind}, workloads::noop::NoopOptions};

use crate::app;

pub fn run(app: InstallyApp, settings: WorkloadKind, do_spawn_ui: bool) -> Executor {
    let executor = instally_core::factory::run(app.clone(), settings, None);
    
    if do_spawn_ui {
        app::create(app).unwrap();
    }

    executor
}

pub fn failed(err: rust_i18n::error::Error) -> Executor {
    run(InstallyApp::default(), WorkloadKind::Error(NoopOptions::default(), err.get_details().clone()), true)
}