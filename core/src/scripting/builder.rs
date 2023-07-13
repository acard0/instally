use once_cell::sync::Lazy;
use rquickjs::{AsyncRuntime, AsyncContext, async_with, FromJs, CatchResultExt, promise::Promise, Object};

use crate::{workloads::abstraction::InstallyApp, extensions::future::FutureSyncExt};

use super::{j_object::{js_app::InstallerJ, JsApp}, error::IJSError};

const IJS_RUNTIME: Lazy<IJSRuntime> = Lazy::new(|| IJSRuntimeContainerBuilder::new().build());

struct IJSRuntimeContainerBuilder {}
impl IJSRuntimeContainerBuilder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn build(&self) -> IJSRuntime {
        IJSRuntime { rt: Box::into_raw(Box::new(AsyncRuntime::new().unwrap())) }
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct IJSRuntime {
    rt: *const AsyncRuntime,
}
unsafe impl Sync for IJSRuntime {}
unsafe impl std::marker::Send for IJSRuntime {}

#[derive(Clone)]
#[repr(transparent)]
pub struct IJSContext {
    ctx: *const AsyncContext,
}
unsafe impl Sync for IJSContext {}
unsafe impl std::marker::Send for IJSContext {}

impl IJSRuntime { 
    pub fn current_or_get() -> Self {
        IJS_RUNTIME.clone()
    }

    pub fn get_runtime(&self) -> AsyncRuntime {
        unsafe { (*self.rt).clone() }
    }

    pub fn create_context(&self, app: InstallyApp) -> IJSContext {
        IJSContext::new(&self, app)
    }

    pub fn free(&self) {
        unsafe { _= Box::from_raw(self.rt as *mut AsyncRuntime); }
    }
}

impl IJSContext {
    pub fn new(rt: &IJSRuntime, app: InstallyApp) -> Self {
        let rt = &rt.get_runtime();
        let ctx = AsyncContext::full(rt).wait().unwrap();

        ctx.with(|ctx| {
            let global = ctx.globals();
            let app_ptr = Box::into_raw(Box::new(app.clone())) as u64;
            let j_object = InstallerJ::new(app_ptr);

            global.init_def::<Sleep>().unwrap();
            global.init_def::<Print>().unwrap();
            global.init_def::<JsApp>().unwrap();
            global.set("Installer", j_object).unwrap();

            // set alias keys
            for (path, value) in app.get_product().create_formatter().iter() {
                let parts: Vec<&str> = path.split('.').collect();
                let mut current_obj = global.clone();
        
                for (i, part) in parts.iter().enumerate() {
                    if i < parts.len() - 1 {
                        let next_obj = match current_obj.get::<_, Object>(part.to_owned()) {
                            Ok(obj) => obj,
                            _ => {
                                let new_obj = Object::new(ctx).unwrap();
                                current_obj.set(part.to_owned(), new_obj.clone()).unwrap();
                                new_obj
                            }
                        };
                        current_obj = next_obj;
                    } else {
                        current_obj.set(part.to_owned(), value.to_owned()).unwrap();
                    }
                }
            }
        }).wait();

        IJSContext { ctx: Box::into_raw(Box::new(ctx)) }
    }

    pub fn get_context(&self) -> AsyncContext {
        unsafe { (*self.ctx).clone() }
    }

    pub fn get_installer_j(&self) -> Result<InstallerJ, IJSError> {
        self.get_context()
        .with(|ctx| { 
            let globals = ctx.globals();
            globals.get::<_, InstallerJ>("Installer")
        }).wait()
        .map_err(|err| IJSError::Other(format!("{err:?}")))
    }
 
    pub fn mount(&self, src: &str) -> Result<(), IJSError> {
        self.eval_raw(src)?;
        self.eval_raw::<()>("mounted();")
            .map_err(|err| IJSError::Other(format!("Failed to mount given script. It has to contain 'mounted' function. {err:?}")))
    }

    pub fn try_eval<V: for<'js> FromJs<'js> + 'static>(&self, src: &str) -> Result<V, IJSError> {
        self.eval_async(&r#"
            try {
                #{inner}#
            } catch (err) {}
        "#.replace("#{inner}#", src))
    }

    pub fn try_eval_raw<V: for<'js> FromJs<'js> + 'static>(&self, src: &str) -> Result<V, IJSError> {
        self.eval_raw(&r#"
            try {
                #{inner}#
            } catch (err) {}
        "#.replace("#{inner}#", src))
    }

    pub fn eval_async<V: for<'js> FromJs<'js> + 'static>(&self, src: &str) -> Result<V, IJSError> {
        let src = r#"
            async function main() {
                #{inner}#
            }

            main()
        "#.replace("#{inner}#", src);

        self.eval_raw_async(&src)
    }

    pub fn eval_raw_async<V: for<'js> FromJs<'js> + 'static>(&self, src: &str) -> Result<V, IJSError> { 
        async_with!(self.get_context() => |ctx| {
            let promise: Promise<V> = ctx.eval(src).catch(ctx)
                .map_err(|err| format!("{err:?}"))?;

            promise.await.catch(ctx)
                .map_err(|err| format!("{err:?}"))
        }).wait() // could use parallel feature of rquickjs but it causes whole a lot of other problems
        .map_err(|err| IJSError::Other(err)) 
    }

    pub fn eval_raw<V: for<'js> FromJs<'js> + 'static>(&self, src: &str) -> Result<V, IJSError> { 
        async_with!(self.get_context() => |ctx| {
            ctx.eval(src).catch(ctx)
                .map_err(|err| IJSError::Other(format!("{err:?}")))
        }).wait()
    }

    pub fn free(&self) {
        unsafe { _= Box::from_raw(self.ctx as *mut AsyncContext); }
    }
}

#[rquickjs::bind(object)]
async fn sleep(msecs: u64) -> rquickjs::Result<()> {
    std::thread::sleep(std::time::Duration::from_millis(msecs));
    Ok(())
}

#[rquickjs::bind(object)]
fn print(msg: String) {
    log::info!("IJS RUNTIME: {msg:?}");
}