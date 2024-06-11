use crate::scripting::{builder::{IJSContext, IJSRuntime}, error::IJSError};

use super::{app::InstallyApp, error::ScriptError, package::Package};

#[derive(Clone, Debug)]
pub struct Script {
    ctx: IJSContext,
}

impl Script {
    pub fn new(src: String, app: &InstallyApp, target_package: Option<&Package>) -> Result<Script, IJSError> {
        let rt = IJSRuntime::current_or_get();
        let ctx = rt.create_context(&app, target_package);
        ctx.mount(&src)?;
        Ok(Script { ctx })
    }

    pub fn invoke_before_installition(&self) -> Result<(), IJSError> { 
        self.ctx.eval_raw::<()>("Installer.on_before_installition();").map_err(|err| IJSError::Execution(format!("{:?}", err)))
    }

    pub fn invoke_after_installition(&self) -> Result<(), IJSError> { 
        self.ctx.eval_raw::<()>("Installer.on_after_installition();").map_err(|err| IJSError::Execution(format!("{:?}", err)))
    }

    pub fn invoke_before_update(&self)  -> Result<(), IJSError> {
        self.ctx.eval_raw::<()>("Installer.on_before_update();").map_err(|err| IJSError::Execution(format!("{:?}", err)))
    }

    pub fn invoke_after_update(&self) -> Result<(), IJSError> { 
        self.ctx.eval_raw::<()>("Installer.on_after_update();").map_err(|err| IJSError::Execution(format!("{:?}", err)))
    }

    pub fn invoke_before_uninstallition(&self) -> Result<(), IJSError> {
        self.ctx.eval_raw::<()>("Installer.on_before_uninstallition();").map_err(|err| IJSError::Execution(format!("{:?}", err)))
    }

    pub fn invoke_after_uninstallition(&self) -> Result<(), IJSError> {
        self.ctx.eval_raw::<()>("Installer.on_after_uninstallition();").map_err(|err| IJSError::Execution(format!("{:?}", err)))
    }
    
    pub fn free(&self) {
        self.ctx.free();
    }
}

pub trait ScriptOptional {
    fn if_exist<F: FnOnce(&Script) -> Result<(), ScriptError>>(&self, action: F) -> Result<(), ScriptError>;
}

impl ScriptOptional for Option<Script> {
    fn if_exist<F: FnOnce(&Script) -> Result<(), ScriptError>>(&self, action: F) -> Result<(), ScriptError> {
        if let Some(script) = self {
            return action(script);
        }

        Ok(())
    }
}