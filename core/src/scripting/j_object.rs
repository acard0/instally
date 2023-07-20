
use rquickjs::{FromJs, Ctx, Value};

#[rquickjs::bind(object, public)]
#[quickjs(bare)]
pub mod js_app {
    use crate::{workloads::abstraction::InstallyApp, extensions::future::FutureSyncExt, target::{GlobalConfig, GlobalConfigImpl}, *};

    pub struct InstallerJ {
        #[quickjs(readonly)]
        pub app_raw_ptr: u64
    }

    impl InstallerJ {
        pub fn new(app_raw_ptr: u64) -> Self {
            Self {
                app_raw_ptr
            }
        }

        #[quickjs(get)]
        pub fn get_progress(&self) -> f32 {
            let binding = self.traverse_app().get_context();
            let ctx = binding.lock();
            ctx.get_progress()
        }

        #[quickjs(get)]
        pub fn get_state(&self) -> String {
            let binding = self.traverse_app().get_context();
            let ctx = binding.lock();
            ctx.get_state_information()
        }

        #[quickjs(get)]
        pub fn get_result(&self) -> String {
            let binding = self.traverse_app().get_context();
            let ctx = binding.lock();
            ctx.get_result().map(|p| { format!("{:?}", p)} ).unwrap_or("".to_string())
        }

        pub fn create_link(&self, original: String, link_dir: String, link_name: String) {
            let _ = self.traverse_app().symlink_file(original, link_dir, &link_name);
        }

        pub fn get_and_execute(&self, url: String, arguments: rquickjs::Array<'_>, state_text: String) -> rquickjs::Result<bool> { 
            let arguments = arguments.iter::<String>()
                .map(|f| format!("{}", f.unwrap()))
                .collect::<Vec<String>>();

            let app = self.traverse_app();
            let dependency_result = app.get_dependency(&url, &state_text).wait();

            if let Ok(dependency) = dependency_result {
                match dependency.execute(arguments, true) {
                    Ok(_) => return Ok(true),
                    Err(err) => {
                        log::trace!("Failed to execute dependency {}", err);
                        return Ok(false);
                    }
                }
            }

            Ok(true)
        }

        pub fn read_reg(&self, key: String, name: String) -> rquickjs::Result<String> {
            GlobalConfig::new().get(key, name).map_err(|err| rquickjs::Error::new_from_js_message(
                "Global config",
                "String",
                format!("Failed to read global config {}", err)
            ))
        }

        pub fn set_reg(&self, key: String, name: String, value: String) -> rquickjs::Result<()> {
            GlobalConfig::new().set(key, name, value).map_err(|err| rquickjs::Error::new_from_js_message(
                "Global config",
                "String",
                format!("Failed to set global config {}", err)
            ))
        }

        pub fn delete_reg(&self, key: String) -> rquickjs::Result<()> {
            GlobalConfig::new().delete(key).map_err(|err| rquickjs::Error::new_from_js_message(
                "Global config",
                "String",
                format!("Failed to delete global config {}", err)
            ))
        }
        
        pub fn command_attached(&self, command: String, arguments: rquickjs::Array<'_>) -> rquickjs::Result<String> {
            let mut cmd = self.create_command(command, arguments);

            cmd.output()
            .map_err(|err| rquickjs::Error::new_from_js_message(
                "Command",
                "String",
                format!("Failed to execute command {}", err)
            ))
            .and_then(|output| {
                let stdout = String::from_utf8(output.stdout).unwrap_or("".to_owned());
                log::trace!("Command {:?} executed with output {:?}", cmd, stdout);
                Ok(stdout)
            })
        }

        pub fn try_command(&self, command: String, arguments: rquickjs::Array<'_>, attached: bool) -> rquickjs::Result<bool> {
            let mut cmd = self.create_command(command, arguments);
            Ok(cmd.output().is_ok())
        }

        pub fn translate(&self, key: String) -> rquickjs::Result<String> {
            Ok(t!(&key))
        }

        #[quickjs(rename = "translate")]
        pub fn translate_with_args(&self, key: String, arguments: rquickjs::Array<'_>) -> rquickjs::Result<String> {
            Ok(t!(&key, arguments.iter::<String>().map(|f| f.unwrap()).collect::<Vec<String>>()))
        }

        #[quickjs(rename = "translate")]
        pub fn translate_from_locale(&self, locale: String, key: String) -> rquickjs::Result<String> {
            Ok(t!(&key, locale = &locale, [""]))
        }    

        #[quickjs(rename = "translate")]
        pub fn translate_from_locale_with_args(&self, locale: String, key: String, arguments: rquickjs::Array<'_>) -> rquickjs::Result<String> {
            Ok(t!(&key, locale = &locale, arguments.iter::<String>().map(|f| f.unwrap()).collect::<Vec<String>>()))
        } 

        pub fn add_translation(&self, locale: String, key: String, value: String) -> rquickjs::Result<()> {
            t_add!(&locale, &key, &value);
            Ok(())
        }

        // Event definitions
        pub fn on_before_installition(&self) { }
        pub fn on_after_installition(&self) { }
        pub fn on_before_update(&self) { }
        pub fn on_after_update(&self) { }
        pub fn on_before_uninstallition(&self) { }
        pub fn on_after_uninstallition(&self) { }
        
        #[quickjs(skip)]
        fn traverse_app(&self) -> InstallyApp {
            let app = unsafe { &mut *(self.app_raw_ptr as *mut InstallyApp) };
            app.clone()
        }

        #[quickjs(skip)]
        fn create_command(&self, command: String, arguments: rquickjs::Array<'_>) -> std::process::Command {
            let arguments = arguments.iter::<String>()
                .map(|f| format!("{}", f.unwrap()))
                .collect::<Vec<String>>();

            let mut cmd = std::process::Command::new(command);
            cmd.args(arguments);
            cmd
        }

        #[quickjs(skip)]
        pub fn free(&self) {
            unsafe { _ = Box::from_raw(self.app_raw_ptr as *mut InstallyApp) };
        }
    }
}

impl<'js> FromJs<'js> for js_app::InstallerJ {
    fn from_js(_: Ctx<'js>, value: Value<'js>) -> rquickjs::Result<Self> {
        let j_obj = value.as_object();
        match j_obj {
            Some(obj) => {
                let app_raw_ptr = obj.get::<_, u64>("app_raw_ptr")
                .map_err(|err| rquickjs::Error::new_from_js_message(
                    "InstallerJ",
                    "InstallerJ (Rust)",
                    format!("Failed to get app_raw_ptr: {}", err)
                ))?;

                return Ok(Self::new(app_raw_ptr));
            },
            _ => {
                return Err(rquickjs::Error::new_from_js("InstallerJ","InstallerJ"));
            }
        }
    }
}