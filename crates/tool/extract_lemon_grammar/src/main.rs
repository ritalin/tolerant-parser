use std::path::PathBuf;

pub fn main() -> Result<(), anyhow::Error> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let build_dir = manifest_dir.join("../../../build");
    let grammar_file_path = build_dir.join("sqlite/src/parse.y").display().to_string();

    let builder = extract_lemon_grammar::lemon::LemonBuilder::new();
    let mut lemon = builder
        .set_args(std::env::args())
        .set_grammar(&grammar_file_path)
        .build()
    ;

    lemon.parse();

    'export_symbol: {
        let symbols = lemon.symbols();
        let grammar_file_path = build_dir.join("grammar_symbols.json");

        std::fs::write(grammar_file_path, serde_json::to_string_pretty(&symbols)?)?;
        break 'export_symbol;
    }
    'export_parsing_rule: {
        let rules = lemon.rules();
        let grammar_file_path = build_dir.join("grammar_parse_rules.json");

        std::fs::write(grammar_file_path, serde_json::to_string_pretty(&rules)?)?;
        break 'export_parsing_rule;
    }
    'export_scanning_rule: {
        std::fs::copy(manifest_dir.join("src/assets/scan_rules.json"), build_dir.join("grammar_scan_rules.json"))?;
        break 'export_scanning_rule;
    }
    Ok(())
}

