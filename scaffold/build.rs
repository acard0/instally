
fn main(){
    if cfg!(target_os = "windows") {
        let ptr_width = std::env::var("CARGO_CFG_TARGET_POINTER_WIDTH")
            .expect("CARGO_CFG_TARGET_POINTER_WIDTH not set");

        let mut res = winres::WindowsResource::new();
        
        if ptr_width == "32" {
            res.set_manifest(r#"
            <assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
            <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
            <security>
            <requestedPrivileges>
            <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
            </requestedPrivileges>
            </security>
            </trustInfo>
            </assembly>
            "#);
        }
        
        res.set_icon("./icons/icons8-setup-64.ico");
        res.compile().unwrap(); 
        
        static_vcruntime::metabuild();
    }
}