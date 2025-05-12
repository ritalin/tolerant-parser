use std::{ffi::OsString, path::PathBuf};

pub fn main() -> Result<(), anyhow::Error> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let generated_dir = manifest_dir.join("src/_generated");

    let needles = std::collections::HashSet::<OsString>::from_iter([
        "scan_rule.rs".into(),
        "symbol_set.rs".into(),
    ]);

    let generated = std::fs::read_dir(generated_dir)?
        .filter_map(|x| match x {
            Ok(entry) => {
                Some(entry.file_name())
            }
            Err(_) => None,
        })
        .all(|name| needles.contains(&name))
    ;

    if generated {
        println!("cargo:rustc-cfg=engine_generated");
    }
    println!("cargo::rustc-check-cfg=cfg(engine_generated)");

    Ok(())
}