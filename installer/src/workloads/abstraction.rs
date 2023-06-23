
use std::{fmt::Display, error::Error, sync::Arc};
use async_trait::async_trait;
use parking_lot::{Mutex};

use crate::http::client;

#[derive(Clone)]
pub enum WorkloadResult {
    Ok,
    Error(String)
}

#[derive(Clone)]
pub struct AppContext<TState>
where TState: Display + Send + Clone + 'static {
    pub frame_count: i32,
    
    state: Option<TState>,
    state_progress: f32,
    result: Option<WorkloadResult>, 
}

#[derive(Clone)]
pub struct InstallyApp<TState>
where TState: Display + Send + Clone + 'static {
    context: Arc<Mutex<AppContext<TState>>>,
}

pub trait ContextAccessor<TState>
where TState: Display + Send + Clone + 'static {
    fn get_context(&self) -> Arc<Mutex<AppContext<TState>>>;
}

#[async_trait]
pub(crate) trait Workload<TState>
where TState: Display + Send + 'static {
    async fn run(&self) -> WorkloadResult;
}

#[async_trait]
pub(crate) trait Worker<TState>: Workload<TState> + ContextAccessor<TState>
where TState: Display + Send + Clone + 'static {
    fn set_workload_state(&self, n_state: TState) {
        self.get_context().lock().state = Some(n_state)
    }

    fn set_state_progress(&self, n_progress: f32) {
        self.get_context().lock().state_progress = n_progress;
    }

    async fn get_file(&self, url: &str, path: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let progress_closure = self.create_progress_closure(url.to_string());
        client::get_file(url, path, progress_closure).await
    }
    
    async fn get_text(&self, url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let progress_closure = self.create_progress_closure(url.to_string());
        client::get_text(url, progress_closure).await
    }

    fn create_progress_closure(&self, url: String) -> Box<dyn FnMut(f32) + Send> {
        let arc = self.get_context();
        Box::new(move |progress: f32| {
            arc.lock().state_progress = progress; 
        })
    }
}

impl<TState> InstallyApp<TState>
where TState: Display + Send + Clone + 'static {
    fn get_context(&self) -> Arc<Mutex<AppContext<TState>>> {
        self.context.clone()
    }
}

impl<TState> ContextAccessor<TState> for InstallyApp<TState>
where TState: Display + Send + Clone + 'static {
    fn get_context(&self) -> Arc<Mutex<AppContext<TState>>> {
        self.context.clone()
    }
}


impl<TState> AppContext<TState>
where TState: Display + Send + Clone + 'static {
    pub fn is_completed(&self) -> bool {
        self.get_result().is_some()
    }

    pub fn is_error(&self) -> bool {
        match self.get_result() {
            Some(WorkloadResult::Error(err)) => true,
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

    pub fn get_state(&self) -> Option<TState> {
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
}

impl<TState> Default for InstallyApp<TState>
where TState: Display + Send + Clone + 'static {
    fn default() -> Self {
        InstallyApp { 
            context: Arc::new(Mutex::new(AppContext::default()))
        }
    }
}

impl<TState> Default for AppContext<TState>
where TState: Display + Send + Clone + 'static {
    fn default() -> Self {
        AppContext {
            frame_count: 0,
            state_progress: 0.0,
            state: None,
            result: None,
        }
    }
}

impl WorkloadResult {
    pub fn is_ok(&self) -> bool {
        match self {
            Self::Ok => true,
            _ => false
        }
    }

    pub fn get_error(&self) -> Option<String> {
        match self {
            Self::Error(err) => Some(err.clone()),
            _ => None,
        }
    }
}
