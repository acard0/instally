use std::{collections::HashMap, sync::Arc};

use once_cell::sync::Lazy;
use parking_lot::Mutex;

use crate::workloads::workload::WorkloadResult;

use super::{app::InstallyApp, summary::InstallationSummary};

pub type ArcM<T> = Arc<Mutex<T>>;
pub type LazyArcM<T> = Lazy<ArcM<T>>;

pub type ContextArcM = ArcM<AppContext>;

static CONTEXT_CALLBACKS: LazyArcM<HashMap<usize, StateCallbackBox>> = LazyArcM::new(|| ArcM::new(Mutex::new(HashMap::new())));

#[derive(Clone, Debug)]
pub struct AppWrapper<T: Default + Clone> {
    pub app: InstallyApp,
    pub settings: T, 
}

impl<T: Default + Clone> AppWrapper<T> {
    pub fn new(app: InstallyApp) -> Self {
        AppWrapper { 
            app,
            settings: T::default()
        }
    }

    pub fn new_with_opts(app: InstallyApp, settings: T) -> Self {
        AppWrapper { app, settings}
    }
}

#[derive(struct_field::StructField, Clone, Debug, Default)]
pub struct AppContext {
    state: Option<String>,
    state_progress: f32,
    result: Option<WorkloadResult>,
    summary: InstallationSummary
}

impl AppContextNotifiable for AppContext {
    fn on_update(&self, field: AppContextField) {
        CONTEXT_CALLBACKS.lock().iter().for_each(|f| {
            let (_, callback) = f;
            callback(AppContextChange { state_cloned: self.clone(), field_cloned:  field.clone()})
        })
    }

    fn subscribe(&self, action: StateCallbackBox) -> usize {
        let mut map =  CONTEXT_CALLBACKS.lock();
        let id = map.len();
        map.insert(id, action);
        id
    }

    fn unsubscribe(&self, id: usize) -> bool {
        CONTEXT_CALLBACKS.lock().remove(&id).is_some()
    }
}

impl AppContext
{
    pub fn new(summary: InstallationSummary) -> Self {
        AppContext {
            state_progress: 0.0,
            state: None,
            result: None,
            summary
        }
    }

    pub fn is_completed(&self) -> bool {
        self.get_result().is_some()
    }

    pub fn is_error(&self) -> bool {
        match self.get_result() {
            Some(WorkloadResult::Error(_)) => true,
            _ => false
        }
    }

    pub fn get_state_information(&self) -> String {
        self.get_state_information_fallback("")
    }

    pub fn get_state_information_fallback(&self, fallback: &str) -> String {
        match self.get_state() {
            None => fallback.to_owned(),
            Some(str) => str.to_string()
        }
    }

    pub fn get_state(&self) -> Option<String> {
        match &self.state {
            Some(st) => Some(st.clone()),
            _ => None
        }
    }

    pub fn get_result(&self) -> Option<WorkloadResult> {
        match &self.result {
            Some(st) => Some(st.clone()),
            _ => None
        }
    }

    pub fn get_progress(&self) -> f32 {
        self.state_progress
    }  

    /// Retreives immutable, cloned instance of 'InstallationSummary'
    pub fn get_summary(&self) -> InstallationSummary {
        self.summary.clone()
    }

    /// Retreives mutable reference to 'InstallationSummary'
    pub fn get_summary_mut(&mut self) -> &mut InstallationSummary {
        &mut self.summary
    }
}