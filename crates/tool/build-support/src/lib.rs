use std::{io::Write, path::{Path, PathBuf}};

pub fn get_tool_build_dir() -> Result<PathBuf, anyhow::Error> {
    let path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?).join("../../../build");

    Ok(path)
}

pub fn download_sqlite_repository(build_dir: &PathBuf) -> Result<PathBuf, anyhow::Error> {
    let dest_dir = build_dir.join("sqlite");

    if ! dest_dir.exists() {
        let path_str = dest_dir.display().to_string();
        std::process::Command::new("git")
            .arg("clone")
            .args(&["--depth=1", "--branch=master", "https://github.com/sqlite/sqlite.git", &path_str])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .output()?
        ;
    }

    Ok(dest_dir)
}

pub fn generate_keyword_check(build_dir: &PathBuf, sqlite_dir: &PathBuf, output_dir: &PathBuf) -> Result<(), anyhow::Error> {
    let generated_command = generate_keywordhash_header(build_dir, sqlite_dir)?;
    let partial_header = run_mkkeywordhash(&build_dir, &generated_command)?;

    let generated_file = merge_keywordhash(&build_dir, &partial_header)?;
    let (_, _, lib_name) = build_keyword_check(&generated_file, output_dir)?;

    println!("cargo:rustc-link-lib=static={}", &lib_name);

    Ok(())
}

fn run_mkkeywordhash(build_dir: &PathBuf, command_path: &PathBuf) -> Result<PathBuf, anyhow::Error> {
    let out_file_path = build_dir.join("keywordhash-partial.h");
    run_command_with_redirect(&command_path.display().to_string(), Some(out_file_path.display().to_string()))?;

    Ok(out_file_path)
}

fn merge_keywordhash(build_dir: &PathBuf, partial_file: &PathBuf) -> Result<PathBuf, anyhow::Error> {
    let out_file_path = build_dir.join("keyword_check.c");
    merge_files(
        include_str!("assets/keywords.h"), 
        &[&partial_file.display().to_string()], 
        &out_file_path
    )?;

    Ok(out_file_path)
}

fn merge_files(base_asset: &str, append_files: &[&str], out_file_path: &PathBuf) -> Result<(), anyhow::Error> {
    let out_file = std::fs::File::create(out_file_path).map_err(|_| anyhow::anyhow!("Can not create file: `{out_file_path:?}`"))?;
    let mut writer = std::io::BufWriter::new(out_file);

    writer.write_all(base_asset.as_bytes())?;

    use std::io::BufRead;
    for file_path in append_files {
        let in_file = std::fs::File::open(file_path).map_err(|_| anyhow::anyhow!("Can not open merge source: `{file_path}`"))?;
        let reader = std::io::BufReader::new(in_file);
        for line in reader.lines() {
            let line = line?; 
            writeln!(writer, "{}", line)?; 
        }
    }

    Ok(())
}

fn generate_keywordhash_header(build_dir: &PathBuf, sqlite_dir: &PathBuf) -> Result<PathBuf, anyhow::Error> {
    let source_path = sqlite_dir.join("tool/mkkeywordhash.c");
    let out_file_path = build_dir.join("mkkeywordhash");

    run_build_c(&source_path.display().to_string(), &["-D", "SQLITE_ASCII=1"], &out_file_path.display().to_string())?;
    Ok(out_file_path)
}

fn build_keyword_check(scource_path: &PathBuf, output_dir: &PathBuf) -> Result<(PathBuf, PathBuf, String), anyhow::Error> {
    let (obj_path, lib_path) = run_build_lib(&scource_path.as_path(), &[], &output_dir, "keyword_check")?;

    let lib_name: String = obj_path.file_stem().expect("Need to generate *.a").to_string_lossy().into();

    Ok((obj_path, lib_path, lib_name))
}

fn run_build_lib(source_path: &Path, additional_options: &[&str], out_dir: &PathBuf, out_filename: &str) -> Result<(PathBuf, PathBuf), anyhow::Error> {
    eprintln!("run_build_lib called !");
    println!("cargo:rerun-if-changed={}", source_path.display());

    let obj_path = out_dir.join(format!("{out_filename}.o"));
    let lib_path = out_dir.join(format!("lib{out_filename}.a"));

    match std::fs::remove_file(&lib_path) {
        Ok(_) => {},
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {},
        Err(err) => return Err(anyhow::anyhow!("{}", err))
    }

    run_build_c_internal(&["-c", &source_path.display().to_string()], additional_options, &obj_path.display().to_string())
    .map_err(|_| anyhow::anyhow!("Failed to compile {}", source_path.display()))?;

    run_ar_obj(&[&obj_path.display().to_string()], &lib_path.display().to_string())?;

    Ok((obj_path, lib_path))
}

fn run_build_c(source_path: &str, include_files: &[&str], out_path: &str) -> Result<(), anyhow::Error> {
    eprintln!("run_build_c called !");
    println!("cargo:rerun-if-changed={source_path}");

    run_build_c_internal(&[source_path], include_files, out_path)
    .map_err(|_| anyhow::anyhow!("Failed to compile {source_path}"))?;

    Ok(())
}

fn run_build_c_internal(source_path: &[&str], additional_options: &[&str], out_path: &str) -> Result<(), anyhow::Error> {
    let status = std::process::Command::new("zig")
        .arg("cc")
        .args(source_path)
        .args(&["-o", out_path])
        .args(additional_options)
        .status()?
    ;
    assert!(status.success(), "Compilation failed");

    Ok(())
}

fn run_ar_obj(obj_files: &[&str], out_path: &str) -> Result<(), anyhow::Error> {
    let status = std::process::Command::new("zig")
        .args(&["ar", "rcs"])
        .arg(out_path)
        .args(obj_files)
        .status()?
    ;
    assert!(status.success(), "Archive failed");

    Ok(())
}

fn run_command_with_redirect(command_path: &str, redirect_path: Option<String>) -> Result<(), anyhow::Error> {
    let mut command = std::process::Command::new(&command_path);

    if let Some(redirect_path) = redirect_path.as_ref() {
        let file = std::fs::File::create(redirect_path).map_err(|_| anyhow::anyhow!("Failed to create {redirect_path}"))?;
        command.stdout(file);
    }

    let mut child = command.spawn().map_err(|_| anyhow::anyhow!("Failed to execute {command_path}"))?;
    let status = child.wait().map_err(|_| anyhow::anyhow!("Failed to wait for gen_code"))?;

    assert!(status.success(), "{command_path} execution failed");

    Ok(())
}

#[cfg(test)]
mod build_tests {
    use super::*;

    #[test]
    fn test_download_sqlite_repository() -> Result<(), anyhow::Error> {
        let dest_dir = download_sqlite_repository(&get_tool_build_dir()?)?;

        assert!(dest_dir.join(".git").exists());

        Ok(())
    }

    #[test]
    fn test_generate_keywordhash() -> Result<(), anyhow::Error> {
        let build_dir = get_tool_build_dir()?;
        let command_path = generate_keywordhash_header(&build_dir, &build_dir.join("sqlite"))?;
        assert!(command_path.exists());

        let file_path = run_mkkeywordhash(&build_dir, &command_path)?;
        assert!(file_path.exists());

        let file_path = merge_keywordhash(&build_dir, &file_path)?;
        assert!(file_path.exists());

        let (obj_file, lib_file, lib_name) = build_keyword_check(&file_path, &build_dir)?;

        assert!(obj_file.exists());
        assert_eq!(Some("o"), obj_file.extension().and_then(|ext| ext.to_str()));
        assert!(lib_file.exists());
        assert_eq!(Some("a"), lib_file.extension().and_then(|ext| ext.to_str()));
        assert_eq!("keyword_check", lib_name);
        Ok(())
    }
}