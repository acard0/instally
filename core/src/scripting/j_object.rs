use std::marker::PhantomData;

use rquickjs::{Array, Error as JsError, JsLifetime, Result as JsResult};
use rquickjs::class::{Trace, Tracer};

use definitions::package::Package;
use crate::target::GlobalConfigImpl;
use crate::{definitions::app::InstallyApp, extensions::future::FutureSyncExt, target::GlobalConfig, *};

#[derive(JsLifetime, Clone)]
#[rquickjs::class(rename_all = "camelCase")]
pub struct InstallerJ<'js> {
    _marker: PhantomData<&'js ()>,
    app_raw_ptr: usize,
    target_package_raw_ptr: usize,
}

impl<'js> Trace<'js> for InstallerJ<'js> {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
}

#[rquickjs::methods]
impl<'js> InstallerJ<'js> {
    #[qjs(constructor)]
    pub fn new(app_raw_ptr: usize, target_package_raw_ptr: usize) -> Self {
        Self { app_raw_ptr, target_package_raw_ptr, _marker: std::marker::PhantomData }
    }

    #[qjs(get)]
    pub fn progress(&self) -> f32 {
        let binding = self.traverse_app().get_context();
        let ctx = binding.lock();
        ctx.get_progress()
    }

    #[qjs(get)]
    pub fn state(&self) -> String {
        let binding = self.traverse_app().get_context();
        let ctx = binding.lock();
        ctx.get_state_information()
    }

    #[qjs(get)]
    pub fn result(&self) -> String {
        let binding = self.traverse_app().get_context();
        let ctx = binding.lock();
        ctx.get_result().map(|p| format!("{:?}", p)).unwrap_or_default()
    }

    pub fn create_link(&self, original: String, link_dir: String, link_name: String) {
        let package = self.traverse_package();
        let _ = self.traverse_app().symlink_file(package, original, link_dir, &link_name);
    }

    pub fn get_and_execute(&self, url: String, arguments: Array<'js>, state_text: String) -> JsResult<bool> {
        let args = arguments.iter::<String>().map(|f| f.unwrap()).collect::<Vec<_>>();
        let app = self.traverse_app();
        let dep = app.get_dependency(&url, &state_text).wait();
        if let Ok(dependency) = dep {
            match dependency.execute(args, true) {
                Ok(_) => return Ok(true),
                Err(err) => {
                    log::trace!("Failed to execute dependency {}", err);
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    pub fn read_reg(&self, key: String, name: String) -> JsResult<String> {
        GlobalConfig::new().get(key, name).map_err(|err| JsError::new_from_js_message(
            "GlobalConfig", "String", format!("Failed to read global config {}", err)
        ))
    }

    pub fn set_reg(&self, key: String, name: String, value: String) -> JsResult<()> {
        GlobalConfig::new().set(key, name, value).map_err(|err| JsError::new_from_js_message(
            "GlobalConfig", "()", format!("Failed to set global config {}", err)
        ))
    }

    pub fn delete_reg(&self, key: String) -> JsResult<()> {
        GlobalConfig::new().delete(key).map_err(|err| JsError::new_from_js_message(
            "GlobalConfig", "()", format!("Failed to delete global config {}", err)
        ))
    }

    pub fn command_attached(&self, command: String, arguments: Array<'js>) -> JsResult<String> {
        let mut cmd = self.create_command(command, arguments);
        cmd.output()
            .map_err(|err| JsError::new_from_js_message(
                "Command", "String", format!("Failed to execute command {}", err)
            ))
            .and_then(|output| {
                let stdout = String::from_utf8(output.stdout).unwrap_or_default();
                log::trace!("Command {:?} executed with output {:?}", cmd, stdout);
                Ok(stdout)
            })
    }

    pub fn try_command(&self, command: String, arguments: Array<'js>, _attached: bool) -> JsResult<bool> {
        let mut cmd = self.create_command(command, arguments);
        Ok(cmd.output().is_ok())
    }

    pub fn translate(&self, key: String) -> rquickjs::Result<String> {
        Ok(t!(&key))
    }

    pub fn translate_with_args(&self, key: String, arguments: rquickjs::Array<'_>) -> rquickjs::Result<String> {
        Ok(t!(&key, arguments.iter::<String>().map(|f| f.unwrap()).collect::<Vec<String>>()))
    }

    pub fn translate_from_locale(&self, locale: String, key: String) -> rquickjs::Result<String> {
        Ok(t!(&key, locale = &locale, [""]))
    }    

    pub fn translate_from_locale_with_args(&self, locale: String, key: String, arguments: rquickjs::Array<'_>) -> rquickjs::Result<String> {
        Ok(t!(&key, locale = &locale, arguments.iter::<String>().map(|f| f.unwrap()).collect::<Vec<String>>()))
    } 

    pub fn add_translation(&self, locale: String, key: String, value: String) -> JsResult<()> {
        t_add!(&locale, &key, &value);
        Ok(())
    }

    #[qjs(skip)] #[allow(dead_code)] fn on_before_installation(&self) {}
    #[qjs(skip)] fn on_after_installation(&self) {}
    #[qjs(skip)] fn on_before_update(&self) {}
    #[qjs(skip)] fn on_after_update(&self) {}
    #[qjs(skip)] fn on_before_uninstallation(&self) {}
    #[qjs(skip)] fn on_after_uninstallation(&self) {}

    #[qjs(skip)]
    fn traverse_app(&self) -> &InstallyApp {
        unsafe { &*(self.app_raw_ptr as *const InstallyApp) }
    }

    #[qjs(skip)]
    fn traverse_package(&self) -> Option<&Package> {
        if self.target_package_raw_ptr == 0 {
            None
        } else {
            Some(unsafe { &*(self.target_package_raw_ptr as *const Package) })
        }
    }

    #[qjs(skip)]
    fn create_command(&self, command: String, arguments: Array<'js>) -> std::process::Command {
        let args = arguments.iter::<String>().map(|f| f.unwrap()).collect::<Vec<_>>();
        let mut cmd = std::process::Command::new(command);
        cmd.args(args);
        cmd
    }

    #[qjs(skip)]
    pub fn free(self) {
        unsafe { _= Box::from_raw(self.app_raw_ptr as *mut InstallyApp); }
    }
}

#[rquickjs::module]
pub mod js_app {
    pub use super::InstallerJ;
}
