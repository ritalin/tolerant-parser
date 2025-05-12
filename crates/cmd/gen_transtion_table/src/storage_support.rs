

pub fn make_output_file(dir_path: std::path::PathBuf, basename: &str) -> Result<std::fs::File, anyhow::Error> {
    let file = std::fs::OpenOptions::new()
        .truncate(true)
        .write(true)
        .create(true)
        .open(dir_path.join(basename))?
    ;

    Ok(file)
}