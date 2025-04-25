use once_cell::sync::Lazy;
use rquickjs::{AsyncRuntime, AsyncContext, async_with, FromJs, CatchResultExt, promise::Promise, Object};

use crate::{definitions::{app::InstallyApp, package::Package}, extensions::future::FutureSyncExt};

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

#[derive(Clone, Debug)]
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

    pub fn create_context(&self, app: &InstallyApp, target_package: Option<&Package>) -> IJSContext {
        IJSContext::new(&self, app.clone(), target_package)
    }

    pub fn free(&self) {
        unsafe { _= Box::from_raw(self.rt as *mut AsyncRuntime); }
    }
}

impl IJSContext {
    pub fn new(rt: &IJSRuntime, app: InstallyApp, target_package: Option<&Package>) -> Self {
        let rt = &rt.get_runtime();
        let ctx = AsyncContext::full(rt).wait().unwrap();

        ctx.with(|ctx| {
            let global = ctx.globals();
            let app_ptr = Box::into_raw(Box::new(app.clone())) as usize;
            let package_ptr = target_package.map(|package| Box::into_raw(Box::new(package.clone())) as usize).unwrap_or_default();
            let j_object = InstallerJ::new(app_ptr, package_ptr);

            global.init_def::<Sleep>().unwrap();
            global.init_def::<Print>().unwrap();
            global.init_def::<Log>().unwrap();
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
        .map_err(|err| IJSError::Execution(format!("{err:?}")))
    }
 
    pub fn mount(&self, src: &str) -> Result<(), IJSError> {
        self.eval_raw(src)?;
        self.eval_raw::<()>("mounted();")
            .map_err(|err| IJSError::Execution(format!("Failed to mount given script. {err:?}")))
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
        .map_err(|err| IJSError::Execution(err)) 
    }

    pub fn eval_raw<V: for<'js> FromJs<'js> + 'static>(&self, src: &str) -> Result<V, IJSError> { 
        async_with!(self.get_context() => |ctx| {
            ctx.eval(src).catch(ctx)
                .map_err(|err| IJSError::Execution(format!("{err:?}")))
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
fn log(msg: String) {
    #[cfg(not(test))]
    log::info!("IJS RUNTIME: {msg:?}");

    #[cfg(test)]
    println!("IJS RUNTIME: {msg:?}");
}

#[rquickjs::bind(object)]
fn print(msg: String) {
    println!("IJS RUNTIME: {msg:?}");
}

#[cfg(test)]
mod tests {
    use crate::definitions::product::Product;

    use super::*;

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
        ).unwrap();

        let app = InstallyApp::build(&product)
            .wait().unwrap();

        let rt = IJSRuntime::current_or_get();
        let ctx = rt.create_context(&app, None);
        
        let _: () = ctx.eval_raw(r#"

            print(`hello: ${Installer.translate("messages.hello")}`);

            Installer.add_translation("de", "messages.hello", "Hallo");
            print(`hello: ${Installer.translate("de", "messages.hello")}`);

            print(`hello: ${Installer.translate("messages.hello.x", ["World"])}`);

            Installer.add_translation("de", "messages.hello.x", "Hallo, {0}");
            print(`hello: ${Installer.translate("de", "messages.hello.x", ["Jason"])}`);

        "#).unwrap();

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

        let _: () = ctx.eval_raw(r#"
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
        
        let _: () = ctx.eval_raw(r#"
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