
pub fn main() -> Result<(), anyhow::Error> {
    let build_dir = build_support::get_tool_build_dir()?;
    let artifact_dir = std::env::var("OUT_DIR")?;
    println!("cargo:rustc-link-search=native={}", artifact_dir);

    // clone sqlite/sqlite
    let sqlite_repo = build_support::download_sqlite_repository(&build_dir)?;

    // // generate static lib
    build_support::generate_keyword_check(&build_dir, &sqlite_repo, &artifact_dir.clone().into())?;
    build_support::generate_lemon_lib(&build_dir, &sqlite_repo, &artifact_dir.into())?;

    Ok(())
}
