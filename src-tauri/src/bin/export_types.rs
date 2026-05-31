use std::fs;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let output = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("src")
        .join("bindings")
        .join("generated.ts");
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, elvpath_lib::typegen::generate_typescript_bindings())?;
    Ok(())
}
