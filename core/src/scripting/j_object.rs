
use rquickjs::{FromJs, Ctx, Value};

#[rquickjs::bind(object, public)]
#[quickjs(bare)]
pub mod js_app {
    use crate::workloads::abstraction::InstallyApp;

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

        pub async fn get_and_execute(&self, url: String, arguments: rquickjs::Array<'_>, state_text: String) -> rquickjs::Result<()> { 
            let arguments = arguments.iter::<String>()
                .map(|f| format!("{}", f.unwrap()))
                .collect::<Vec<String>>();

            let app = self.traverse_app();
            let dependency_result = app.get_dependency(&url, &state_text).await;

            if let Ok(dependency) = dependency_result {
                dependency.execute(arguments, true);
            }

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