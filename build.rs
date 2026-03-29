use std::env;

fn main() {
    let assets = std::fs::read_dir("assets").unwrap();
    for entry in assets {
        let path = entry.unwrap().path();
        if path.is_dir() {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
    println!("cargo:rustc-env=ASSETS_DIR={}", env::var("CARGO_MANIFEST_DIR").unwrap() + "/assets");
}
