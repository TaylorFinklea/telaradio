//! Build script: regenerate `telaradio_ffi.h` whenever the crate's source
//! changes, dropped at a path the Swift package picks up via its
//! `module.modulemap`.

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=cbindgen.toml");
    println!("cargo:rerun-if-changed=build.rs");

    let crate_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set");
    let crate_path = PathBuf::from(&crate_dir);

    // Land the header inside the Swift package's bridging include dir so
    // SwiftPM can pick it up via module.modulemap.
    let header_path = crate_path
        .parent()
        .expect("ffi crate has a parent")
        .join("apple/Telaradio/Sources/TelaradioFFI/include/telaradio_ffi.h");

    if let Some(parent) = header_path.parent() {
        std::fs::create_dir_all(parent).expect("create header include dir");
    }

    let bindings = cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(
            cbindgen::Config::from_file(crate_path.join("cbindgen.toml"))
                .expect("read cbindgen.toml"),
        )
        .generate()
        .expect("cbindgen generate");

    bindings.write_to_file(&header_path);
    println!("cargo:warning=wrote header: {}", header_path.display());
}
