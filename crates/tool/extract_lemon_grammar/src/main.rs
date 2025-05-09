use std::path::PathBuf;

pub fn main() -> Result<(), anyhow::Error> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let build_dir = PathBuf::from(manifest_dir).join("../../../build");
    let grammar_file_path = build_dir.join("sqlite/src/parse.y").display().to_string();

    let builder = extract_lemon_grammar::lemon::LemonBuilder::new();
    let mut lemon = builder
        .set_args(std::env::args())
        .set_grammar(&grammar_file_path)
        .build()
    ;

    lemon.parse();
    let symbols = lemon.symbols();
    let symbol_file_path = build_dir.join("grammar_symbols.json");

    std::fs::write(symbol_file_path, serde_json::to_string_pretty(&symbols)?)?;

    Ok(())
}

