
fn main() {
  if cfg!(target_os = "windows") {
    let mut res = winres::WindowsResource::new();
    res.set_icon("./icons/icons8-setup-64.ico");
    res.compile().unwrap();
  }
}