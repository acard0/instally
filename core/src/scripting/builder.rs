use once_cell::sync::Lazy;
use rquickjs::{context::intrinsic, AsyncContext, CatchResultExt, Context, FromJs, Function, Object, Runtime};

use crate::definitions::{app::InstallyApp, package::Package};

use super::{j_object::js_app::InstallerJ, error::IJSError};

const IJS_RUNTIME: Lazy<IJSRuntime> = Lazy::new(|| IJSRuntimeContainerBuilder::new().build());

struct IJSRuntimeContainerBuilder {}
impl IJSRuntimeContainerBuilder {
    pub fn new() -> Self { Self {} }
    pub fn build(&self) -> IJSRuntime {
        IJSRuntime { rt: Box::into_raw(Box::new(Runtime::new().unwrap())) }
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct IJSRuntime { rt: *const Runtime }
unsafe impl Sync for IJSRuntime {}
unsafe impl Send for IJSRuntime {}

#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct IJSContext { ctx: *const Context }
unsafe impl Sync for IJSContext {}
unsafe impl Send for IJSContext {}

impl IJSRuntime { 
    pub fn current_or_get() -> Self {
        IJS_RUNTIME.clone()
    }

    pub fn get_runtime(&self) -> &Runtime {
        unsafe { &*self.rt }
    }

    pub fn create_context(&self, app: &InstallyApp, target_package: Option<&Package>) -> IJSContext {
        IJSContext::new(&self, app.clone(), target_package)
    }

    pub fn free(&self) {
        unsafe { _= Box::from_raw(self.rt as *mut Runtime); }
    }
}

impl IJSContext {
    pub fn new(rt: &IJSRuntime, app: InstallyApp, target_package: Option<&Package>) -> Self {
        let rt_ref = rt.get_runtime();
        let ctx = Context::builder()
            .with::<intrinsic::All>()
            .build(rt_ref)
            .unwrap();

        ctx.with(|ctx| { 
            let app_ptr = Box::into_raw(Box::new(app.clone())) as usize;
            let package_ptr = target_package.map(|p| Box::into_raw(Box::new(p.clone())) as usize).unwrap_or_default();
            let j_object = InstallerJ::new(app_ptr, package_ptr);
            
            let global = ctx.globals();
            global.set("Installer", j_object).unwrap();
            global.set("sleep", Function::new(ctx.clone(), js_sleep)).unwrap();
            global.set("print", Function::new(ctx.clone(), js_print)).unwrap();
            global.set("log", Function::new(ctx.clone(), js_log)).unwrap();

            ctx.eval::<(), _>(r#"
                Installer.translate = function(...args) {
                    if (args.length === 1)                        return Installer.translate(...args);
                    if (args.length === 2 && Array.isArray(args[1])) return Installer.translate_with_args(...args);
                    if (args.length === 2)                        return Installer.translate_from_locale(...args);
                    if (args.length === 3)                        return Installer.translate_from_locale_with_args(...args);
                    log("Invalid arguments to translate");
                };
            "#).unwrap();

            for (path, value) in app.get_product().create_formatter().iter() {
                let parts: Vec<&str> = path.split('.').collect();
                let mut current = global.clone();
                for (i, part) in parts.iter().enumerate() {
                    if i + 1 < parts.len() {
                        current = match current.get::<_, Object>(*part) {
                            Ok(obj) => obj,
                            Err(_) => {
                                let new_obj = Object::new(ctx.clone()).unwrap();
                                current.set(*part, new_obj.clone()).unwrap();
                                new_obj
                            }
                        };
                    } else {
                        current.set(*part, value.to_owned()).unwrap();
                    }
                }
            }
        });

        IJSContext { ctx: Box::into_raw(Box::new(ctx)) }
    }

    pub fn get_rquickjs_context(&self) -> Context {
        unsafe { (&*self.ctx).clone() }
    }

    pub fn get_installer_j(&self) -> Result<InstallerJ, IJSError> {
        let wrapper = self.get_rquickjs_context().with(|ctx| {
            Ok(Box::into_raw(Box::new(ctx.globals().get::<_, InstallerJ>("Installer")?.clone())) as usize)
        })
        .map(|m_context| *unsafe { Box::from_raw(m_context as *mut InstallerJ) })
        .map_err(|err: rquickjs::Error| {
            IJSError::Execution(format!("{err:?}"))
        })?;
    
        Ok(wrapper)
    }
 
    pub fn mount(&self, src: &str) -> Result<(), IJSError> {
        self.eval::<()>(src)?;
        self.eval::<()>("mounted();")
            .map_err(|err| IJSError::Execution(format!("Failed to mount given script. {err:?}")))
    }   

    pub fn eval<V: for<'js> FromJs<'js> + 'static>(&self, src: &str) -> Result<V, IJSError> { 
        self.get_rquickjs_context()
        .with(|ctx| {
            ctx.eval(src)
                .catch(&ctx)
                .map_err(|err| IJSError::Execution(format!("{err:?}")))
        })
    }

    pub fn free(&self) {
        unsafe { _= Box::from_raw(self.ctx as *mut AsyncContext); }
    }
}

#[rquickjs::function]
async fn sleep(msecs: u64) -> rquickjs::Result<()> {
    std::thread::sleep(std::time::Duration::from_millis(msecs));
    Ok(())
}

#[rquickjs::function]
fn log(msg: String) {
    #[cfg(not(test))]
    log::info!("IJS RUNTIME: {msg:?}");
    #[cfg(test)]
    println!("IJS RUNTIME: {msg:?}");
}

#[rquickjs::function]
fn print(msg: String) {
    println!("IJS RUNTIME: {msg:?}");
}

#[cfg(test)]
mod tests {

    use crate::{definitions::product::Product, extensions::future::FutureSyncExt};

    use super::*;
    
    #[tokio::test]
    async fn test_rquickjs(){
        let product = Product::from_template(
            Product::new(
                "Wulite Beta",
                "@{App.Name}",
                "liteware.io",
                "https://liteware.io",
                "https://cdn.liteware.xyz/downloads/wulite/beta/",
                "global_script.js",
                "@{Directories.User.Home}\\AppData\\Local\\@{App.Publisher}\\@{App.Name}",
            )
        ).expect("failed to build test product");
        let app = InstallyApp::build(&product).await.expect("failed to build instally app");

        let rt = Runtime::new().unwrap();
        let ctx = Context::builder()
            .with::<intrinsic::All>()
            .build(&rt)
            .unwrap();

        let result = ctx.with(|ctx| {
            ctx.eval::<i32, _>(r#"
                331
            "#)
            .catch(&ctx)
            .map_err(|err| IJSError::Execution(format!("{err:?}")))
        });

        println!("{result:?}")
    }

    #[tokio::test]
    async fn test_eval(){
        let product = Product::from_template(
            Product::new(
                "Wulite Beta",
                "@{App.Name}",
                "liteware.io",
                "https://liteware.io",
                "https://cdn.liteware.xyz/downloads/wulite/beta/",
                "global_script.js",
                "@{Directories.User.Home}\\AppData\\Local\\@{App.Publisher}\\@{App.Name}",
            )
        ).expect("failed to build test product");

        let app = InstallyApp::build(&product).await.expect("failed to build instally app");
        let rt = IJSRuntime::current_or_get();
        let ctx = rt.create_context(&app, None);

        let _: () = ctx.eval::<()>(r#"
            print(`hello: ${Installer.translate("messages.hello")}`);

            Installer.add_translation("de", "messages.hello.x", "Hallo, {0}");
            print(`hello: ${Installer.translate("de", "messages.hello.x", ["Jason"])}`);
        "#).unwrap();
    }

    #[test]
    fn test_localization() {
        let product = Product::from_template(
            Product::new(
                "Wulite Beta",
                "@{App.Name}",
                "liteware.io",
                "https://liteware.io",
                "https://cdn.liteware.xyz/downloads/wulite/beta/",
                "global_script.js",
                "@{Directories.User.Home}\\AppData\\Local\\@{App.Publisher}\\@{App.Name}",
            )
        ).expect("failed to build test product");

        let app = InstallyApp::build(&product).wait().expect("failed to build instally app");
        let rt = IJSRuntime::current_or_get();
        let ctx = rt.create_context(&app, None);
        
        let _: () = ctx.eval(r#"

            print(`hello: ${Installer.translate("messages.hello")}`);

            Installer.add_translation("de", "messages.hello", "Hallo");
            print(`hello: ${Installer.translate("de", "messages.hello")}`);

            print(`hello: ${Installer.translate("messages.hello.x", ["World"])}`);

            Installer.add_translation("de", "messages.hello.x", "Hallo, {0}");
            print(`hello: ${Installer.translate("de", "messages.hello.x", ["Jason"])}`);

        "#).expect("failed to eval translation test script");

        ctx.free();
        rt.free();
    }
    
    #[test]
    fn test_install_redistruble_package() -> Result<(), rust_i18n::error::Error> {
        let product = Product::from_template(
            Product::new(
                "Wulite Beta",
                "@{App.Name}",
                "liteware.io",
                "https://liteware.io",
                "https://cdn.liteware.io/downloads/wulite/release/",
                "global_script.js",
                "@{Directories.User.Home}\\AppData\\Roaming\\@{App.Publisher}\\@{App.Name}",
            )
        )?;

        let app = InstallyApp::build(&product)
            .wait()?;

        let rt = IJSRuntime::current_or_get();
        let ctx = rt.create_context(&app, None);

        let _: () = ctx.eval(r#"
            log('Installed OS: ' + System.Os.Name);

            Installer.add_translation("en-US", "states.migrating-installer", "Updating the Installer");
            Installer.add_translation("tr-TR", "states.migrating-installer", "Kurulum güncelleniyor");
        
            if (System.Os.Name === "windows") {
                Installer.get_and_execute("https://cdn.liteware.io/downloads/wulite/release/Setup.exe", [], Installer.translate("states.migrating-installer"));
            }
        "#)?;

        ctx.free();
        rt.free();
        Ok(())
    }

    #[test]
    fn test_dependency_check() {
        let product = Product::from_template(
            Product::new(
                "Wulite Beta",
                "@{App.Name}",
                "liteware.io",
                "https://liteware.io",
                "https://cdn.liteware.xyz/downloads/wulite/beta/",
                "global_script.js",
                "@{Directories.User.Home}\\AppData\\Roaming\\@{App.Publisher}\\@{App.Name}",
            )
        ).unwrap();

        let app = InstallyApp::build(&product)
            .wait().unwrap();

        let rt = IJSRuntime::current_or_get();
        let ctx = rt.create_context(&app, None);
        
        let _: () = ctx.eval(r#"
            log('Installed OS: ' + System.Os.Name);

            if (System.Os.Name === "windows") {
                const webview2_uri = 'https://msedge.sf.dl.delivery.mp.microsoft.com/filestreamingservice/files/6e5c75e2-3d95-4b41-abcb-dc6cef509a32/MicrosoftEdgeWebView2RuntimeInstallerX64.exe';
                const dotnet_desktop_runtime = 'https://download.visualstudio.microsoft.com/download/pr/1146f414-17c7-4184-8b10-1addfa5315e4/39db5573efb029130add485566320d74/windowsdesktop-runtime-6.0.20-win-x64.exe';
                const dotnet_aspnetcore_runtime = "https://download.visualstudio.microsoft.com/download/pr/be9f67fd-60af-45b1-9bca-a7bcc0e86e7e/6a750f7d7432937b3999bb4c5325062a/aspnetcore-runtime-6.0.20-win-x64.exe";
        
                const reAspNet = /Microsoft\.AspNetCore\.App 6\./g;
                const reNetCore = /Microsoft\.NETCore\.App 6\./g;
                const reWindowsDesktop = /Microsoft\.WindowsDesktop\.App 6\./g;

                log('Detected os is Windows. Checking WebView2 installation.');     
                try 
                {
                    log("Checking WebView2 version");
                    var key = "HKEY_CURRENT_USER\\Software\\Microsoft\\EdgeWebView\\BLBeacon";
                    var pv = Installer.read_reg(key, "version");
                    log(`WebView2 version: ${pv}`);
                } catch (err) {
                    log('WebView2 is not installed. Installing...');
                    Installer.get_and_execute(webview2_uri, [ "/silent", "/install" ], Installer.translate("states.installing", ["Microsoft WebView2"]));
                }
                log('WebView2 is installed.');
        
                log('Checking .NET 6 installation.');
                var dotnet_query = "";
                try {
                    dotnet_query = Installer.command_attached('dotnet', ['--list-runtimes'])
                } catch (err) { } 
                
                print(`dotnet query: ${dotnet_query}`);
                var netCore = reNetCore.test(dotnet_query);
                var aspNet = reAspNet.test(dotnet_query);
                var windowsDesktop = reWindowsDesktop.test(dotnet_query);

                if (!netCore || !windowsDesktop) {
                    print(".NET Desktop Runtime 6.* is not installed. Installing...");
                    Installer.get_and_execute(dotnet_desktop_runtime, [ "/install", "/quiet", "/norestart" ], Installer.translate("states.installing", ["Microsoft .NET 6 Desktop Runtime"]));
                } else {
                    log('.NET Desktop Runtime 6.* is installed.');
                } 
                
                if (!aspNet) {
                    print(".NET AspNetCore Runtime 6.* it not installed. Installing...");
                    Installer.get_and_execute(dotnet_aspnetcore_runtime, [ "/install", "/quiet", "/norestart" ], Installer.translate("states.installing", ["Microsoft .NET 6 AspNetCore Runtime"]));
                } else {
                    log('.NET AspNetCore Runtime 6.* is installed.');
                }
            }
        "#).unwrap();

        ctx.free();
        rt.free();
    }
}