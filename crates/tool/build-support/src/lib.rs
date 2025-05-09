use std::path::PathBuf;

pub fn get_tool_build_dir() -> Result<PathBuf, anyhow::Error> {
    let path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?).join("../../../build");

    Ok(path)
}

pub fn download_sqlite_repository(build_dir: &PathBuf) -> Result<PathBuf, anyhow::Error> {
    let dest_dir = build_dir.join("sqlite");

    if ! dest_dir.exists() {
        let path_str = dest_dir.display().to_string();
        std::process::Command::new("git")
            .args(&["clone", "--depth=1", "--branch=master", "https://github.com/sqlite/sqlite.git", &path_str])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .output()?
        ;
    }

    Ok(dest_dir)
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
}