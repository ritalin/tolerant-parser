use std::{io::Write, io::BufWriter, path::PathBuf};
use grammar_types_core::symbol::GrammarSymbol;
use quote::{format_ident, quote};

use crate::export_support::{tokens_to_string, with_indent};



pub fn generate(symbols: &[GrammarSymbol], output_dir: PathBuf) -> Result<(), anyhow::Error> {
    let output_file = crate::storage_support::make_output_file(output_dir, "symbol_set.rs")?;
    let mut writer = BufWriter::new(output_file);

    // generate SyntakKinds
    create_syntax_kind_token(symbols, &mut writer)?;
    // generate id to SyntaxKindMap
    create_syntax_kind_map(symbols, &mut writer)?;

    Ok(())
}

fn create_syntax_kind_token(symbols: &[GrammarSymbol], writer: &mut impl Write) -> Result<(), anyhow::Error> {
    writeln!(writer, "pub mod syntax_kind {{")?;
    writeln!(writer, "{}", with_indent("use engine_core::SyntaxKind;", 1))?;

    for symbol in symbols {
        let ident = format_ident!("r#{}", symbol.name);
        let text = symbol.name.clone();
        let id = symbol.id;
        let is_keyword = if let grammar_types_core::SymbolType::Terminal { is_keyword } = symbol.symbol_type { is_keyword } else { false };
        let is_terminal = if let grammar_types_core::SymbolType::Terminal { .. } = symbol.symbol_type { true } else { false };
    
        let q = quote! {
            #[allow(non_upper_case_globals)] pub static #ident: SyntaxKind = SyntaxKind { text: #text, id: #id, is_keyword: #is_keyword, is_terminal: #is_terminal };
        };

        writeln!(writer, "{}", tokens_to_string(q, 1))?;
    }
    writeln!(writer, "}}")?;

    Ok(())
}

fn create_syntax_kind_map(symbols: &[GrammarSymbol], writer: &mut impl Write) -> Result<(), anyhow::Error> {
    writeln!(writer, "pub mod syntax_map {{")?;
    writeln!(writer, "{}", with_indent("use super::syntax_kind::*;", 1))?;
    writeln!(writer, "{}", with_indent("pub static SYNTAX_KIND_MAP: phf::Map<u32, &'static engine_core::SyntaxKind> = phf::phf_map!{", 1))?;

    for symbol in symbols {
        let ident = format_ident!("r#{}", symbol.name);
        let id = symbol.id;
    
        writeln!(writer, "{}", tokens_to_string(quote! { #id => &#ident, }, 2))?;
    }

    writeln!(writer, "{}", with_indent("};", 1))?;
    writeln!(writer, "}}")?;

    Ok(())
}
