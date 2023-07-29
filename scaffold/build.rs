
fn main() {
  if cfg!(target_os = "windows") {

    // embed icon
    let mut res = winres::WindowsResource::new();
    res.set_icon("./icons/icons8-setup-64.ico");
    res.compile().unwrap();

    /*
      link vcruntime

      could cross-compile instead of this but skia is used as software renderer 
      by egui which is does not support cross-compliation
     */
    static_vcruntime::metabuild();
  }
}