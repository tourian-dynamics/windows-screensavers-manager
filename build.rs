use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=assets/brand/app.ico");
    let ico_path = Path::new("assets/brand/app.ico");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" && ico_path.exists() {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/brand/app.ico");
        res.compile().expect("failed to compile winres resource");
    }
}
