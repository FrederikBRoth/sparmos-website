use std::{env, fs, io, path::Path};

fn copy_tree(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        if entry.file_type()?.is_dir() {
            copy_tree(&source_path, &destination_path)?;
        } else {
            fs::copy(source_path, destination_path)?;
        }
    }

    Ok(())
}

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed=assets");

    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set");
    let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR is set");
    let profile_dir = Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .expect("OUT_DIR has Cargo's target profile layout");

    copy_tree(
        &Path::new(&manifest_dir).join("assets"),
        &profile_dir.join("assets"),
    )
}
