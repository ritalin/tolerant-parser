use std::{collections::HashSet, io::{BufWriter, Write}, path::PathBuf};
use tolerant_parser_sdk::support::grammar_types::{SymbolType, symbol::GrammarSymbol};
use quote::{format_ident, quote};

use crate::export_support::{tokens_to_string, with_indent};



pub fn generate(symbols: &[GrammarSymbol], regex_symbols: &HashSet<&String>, output_dir: PathBuf) -> Result<(), anyhow::Error> {
    let output_file = crate::storage_support::make_output_file(output_dir, "symbol_set.rs")?;
    let mut writer = BufWriter::new(output_file);

    // generate SyntakKinds
    create_syntax_kind_token(symbols, regex_symbols, &mut writer)?;
    // generate id to SyntaxKindMap
    create_syntax_kind_map(symbols, &mut writer)?;

    Ok(())
}

fn create_syntax_kind_token(symbols: &[GrammarSymbol], regex_symbols: &HashSet<&String>, writer: &mut impl Write) -> Result<(), anyhow::Error> {
    writeln!(writer, "pub mod syntax_kind {{")?;
    writeln!(writer, "{}", with_indent("use tolerant_parser_sdk::core::engine_core::SyntaxKind;", 1))?;
    writeln!(writer, "{}", with_indent("use tolerant_parser_sdk::core::engine_core::SymbolGroup;", 1))?;

    for symbol in symbols {
        let ident = format_ident!("r#{}", symbol.name);
        let text = symbol.name.clone();
        let id = symbol.id;

        let group = match symbol.symbol_type {
            SymbolType::Terminal { is_keyword } if is_keyword => quote! { SymbolGroup::Keyword },
            SymbolType::Terminal { .. } if regex_symbols.contains(&symbol.name) => quote! { SymbolGroup::Pattern },
            SymbolType::Terminal { .. } => quote! { SymbolGroup::NonKeyword }, 
            SymbolType::NonTerminal |
            SymbolType::MultiTerminal { .. } => quote! { SymbolGroup::NonTerminal },
        };
    
        let q = quote! {
            #[allow(non_upper_case_globals)] pub static #ident: SyntaxKind = SyntaxKind { text: #text, id: #id, group: #group };
        };

        writeln!(writer, "{}", tokens_to_string(q, 1))?;
    }
    writeln!(writer, "}}")?;

    Ok(())
}

fn create_syntax_kind_map(symbols: &[GrammarSymbol], writer: &mut impl Write) -> Result<(), anyhow::Error> {
    writeln!(writer, "pub mod syntax_map {{")?;

    writeln!(writer, "{}", with_indent("use super::syntax_kind::*;", 1))?;
    writeln!(writer, "{}", with_indent("#[cfg(engine_ungenerated)]", 1))?;
    writeln!(writer, "{}", with_indent("pub static SYNTAX_KIND_MAP: phf::Map<u32, &'static tolerant_parser_sdk::core::engine_core::SyntaxKind> = phf::phf_map!{};", 1))?;
    writeln!(writer, "{}", with_indent("#[cfg(not(engine_ungenerated))]", 1))?;
    writeln!(writer, "{}", with_indent("pub static SYNTAX_KIND_MAP: phf::Map<u32, &'static tolerant_parser_sdk::core::engine_core::SyntaxKind> = phf::phf_map!{", 1))?;

    for symbol in symbols {
        let ident = format_ident!("r#{}", symbol.name);
        let id = symbol.id;
    
        writeln!(writer, "{}", tokens_to_string(quote! { #id => &#ident, }, 2))?;
    }

    writeln!(writer, "{}", with_indent("};", 1))?;
    writeln!(writer, "}}")?;

    Ok(())
}
