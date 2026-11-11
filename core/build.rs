fn main() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let root = std::path::PathBuf::from(&crate_dir)
        .parent()
        .unwrap()
        .to_path_buf();

    let config =
        cbindgen::Config::from_file(std::path::PathBuf::from(&crate_dir).join("cbindgen.toml"))
            .unwrap();

    let generated = std::path::PathBuf::from(&out_dir).join("quantion.h");

    cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(&generated);

    let copies = [
        root.join("core/include/quantion.h"),
        root.join("wrappers/js/src/include/quantion.h"),
        root.join("wrappers/r/src/include/quantion.h"),
    ];

    let is_repository = root.join("wrappers").is_dir();
    for copy in &copies {
        if !is_repository {
            break;
        }
        let parent = copy.parent().unwrap();
        if std::fs::create_dir_all(parent).is_err() {
            continue;
        }
        let _ = std::fs::copy(&generated, copy);
    }

    println!("cargo:rerun-if-changed=cbindgen.toml");
    println!("cargo:rerun-if-changed=src");
}
