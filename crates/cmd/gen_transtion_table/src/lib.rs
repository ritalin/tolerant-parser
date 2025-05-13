mod configs;
mod generator_scan_rule;
mod generator_symbol_set;
mod export_support;
mod storage_support;

use std::{collections::HashMap, path::{Path, PathBuf}};
use grammar_types_core::{scan_rule::GrammarScanRule, symbol::GrammarSymbol};

pub use configs::CmdConfig;

pub fn generate(config: CmdConfig) -> Result<(), anyhow::Error> {
    let base_dir = PathBuf::from(config.source_dir);
    let output_dir = PathBuf::from(config.output_dir);
    let temp_dir = tempfile::tempdir()?;
    let backup_dir = tempfile::tempdir()?;

    let symbols = read_json_file::<Vec<GrammarSymbol>>(base_dir.join(config.grammar_symbol))?;
    let scan_rules = read_json_file::<GrammarScanRule>(base_dir.join(config.grammar_scan_rule))?;

    let symbol_map = symbols.iter().map(|x| (x.name.to_string(), x.id)).collect::<HashMap<_, _>>();
    
    generator_symbol_set::generate(&symbols, temp_dir.path().to_path_buf())?;
    generator_scan_rule::generate(&scan_rules, &symbols, &symbol_map, temp_dir.path().to_path_buf())?;

    swap_folder(&output_dir, temp_dir.path(), &backup_dir.path().join("backup"))?;

    Ok(())
}

fn read_json_file<Content: serde::de::DeserializeOwned>(path: PathBuf) -> Result<Content, anyhow::Error> {
    let file = std::fs::File::open(path)?;
    let content: Content = serde_json::from_reader(file)?;
    Ok(content)
}

fn swap_folder(old_folder: &PathBuf, new_folder: &Path, backup_dir: &Path) -> Result<(), anyhow::Error> {
    if std::fs::exists(old_folder)? {
        std::fs::rename(old_folder, backup_dir)?;
    }
    
    match std::fs::rename(new_folder, old_folder) {
        Ok(_) => {}
        Err(err) => {
            if std::fs::exists(backup_dir)? {
                std::fs::rename(backup_dir, old_folder)?;
            }
            return Err(anyhow::anyhow!(err));
        }
    }

    Ok(())
}