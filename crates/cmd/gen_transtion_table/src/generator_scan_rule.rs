use std::{collections::{BTreeMap, HashMap}, path::PathBuf};
use std::io::{Write, BufWriter};
use grammar_types_core::{scan_rule::{GrammarScanRule, RegexGrammarScanRule}, symbol::GrammarSymbol, SymbolType};
use quote::quote;

use crate::export_support::{tokens_to_string, with_indent};

pub fn generate(rules: &GrammarScanRule, symbols: &[GrammarSymbol], symbol_lookup: &HashMap<String, u32>, output_dir: PathBuf) -> Result<(), anyhow::Error> {
    let output_file = crate::storage_support::make_output_file(output_dir, "scan_rule.rs")?;
    let mut writer = BufWriter::new(output_file);

    writeln!(writer, "mod scan_rule_map {{")?;
    writeln!(writer, "{}", with_indent("use engine_core::scanner_engine::ScanPattern;", 1))?;

    generate_lexme_scan_rule(&rules.lexme, collect_keywords(symbols), symbol_lookup, &mut writer)?;
    generate_regex_scan_rule(&rules.regex, symbol_lookup, &mut writer)?;
    generate_alternative_token(&rules.alternatives, symbol_lookup, &mut writer)?;

    writeln!(writer, "}}")?;

    Ok(())
}

fn collect_keywords(symbols: &[GrammarSymbol]) -> impl Iterator<Item = GrammarSymbol> {
    symbols.iter().filter_map(|symbol| match symbol.symbol_type {
        SymbolType::Terminal { is_keyword } if is_keyword => {
            Some(symbol.clone())
        }
        _ => None,
    })
}

fn generate_lexme_scan_rule(
    lexme: &HashMap<String, Vec<String>>, 
    keywords: impl Iterator<Item = GrammarSymbol>, 
    symbol_lookup: &HashMap<String, u32>,
    writer: &mut impl Write) -> Result<(), anyhow::Error> 
{
    let mut scan_rules = BTreeMap::<char, Vec<String>>::new();
    
    // Prepare keyword prefix map
    for symbol in keywords {
        let rule = tokens_to_string(export_rule_pattern(symbol.id, &symbol.name, symbol.name.len()), 3);
        let (_, prefix) = symbol.name.char_indices().next().unwrap();

        scan_rules.entry(prefix.to_ascii_lowercase())
            .and_modify(|xs| xs.push(rule.clone()))
            .or_insert_with(|| vec![rule])
        ;
    }

    for (name, patterns) in lexme {
        let id = *symbol_lookup.get(name).unwrap();

        for pattern in patterns {
            let rule = tokens_to_string(export_rule_pattern(id, &pattern, pattern.len()), 3);
            let (_, prefix) = pattern.char_indices().next().unwrap();

            scan_rules.entry(prefix.to_ascii_lowercase())
                .and_modify(|xs| xs.push(rule.clone()))
                .or_insert_with(|| vec![rule])
            ;
        }
    }

    writeln!(writer, "{}", with_indent("pub static LEXME_SCAN_RULE: phf::Map<char, &'static [ScanPattern]> = phf::phf_map!{", 1))?;

    for (prefix, mut scan_rule) in scan_rules {
        scan_rule.sort_by(|lhs, rhs| lhs.len().cmp(&rhs.len()).reverse());

        writeln!(writer, "{}", with_indent(&format!("'{prefix}' => &["), 2))?;
        for rule in scan_rule {
            writeln!(writer, "{}", rule)?;
        }
        writeln!(writer, "{}", with_indent("],", 2))?;
    }

    writeln!(writer, "{}", with_indent("};", 1))?;

    Ok(())
}

fn generate_regex_scan_rule(
    regex: &BTreeMap<String, Vec<RegexGrammarScanRule>>, 
    symbol_lookup: &HashMap<String, u32>,
    writer: &mut impl Write) -> Result<(), anyhow::Error> 
{
    let mut scan_rules = vec![];
    let mut support_leading = vec![];
    let mut support_trailing = vec![];
    let mut support_main = vec![];
    let mut i: usize = 0;

    for (name, patterns) in regex {
        let id = *symbol_lookup.get(name).unwrap();
        for pattern in patterns {
            let rule = tokens_to_string(export_rule_pattern(id, &pattern.pattern, pattern.pattern.len()), 1);
            scan_rules.push(rule.clone());

            let support_index = with_indent(&export_rule_support(i, &pattern.pattern), 1);
            if pattern.leading { 
                support_leading.push(support_index.clone()); 
            }
            if pattern.trailing { 
                support_trailing.push(support_index.clone()); 
            }
            if pattern.main { 
                support_main.push(support_index.clone()); 
            }
            i += 1;
        }
    }

    // rexex pattern
    export_vec(
        "pub static REGEX_SCAN_RULE: &[ScanPattern] = &[",
        &scan_rules,
        "];",
        writer
    )?;
    // leading regex support index
    export_vec(
        "pub static SUPPORT_LEADING: &[usize] = &[",
        &support_leading,
        "];",
        writer
    )?;
    // trailing regex support index
    export_vec(
        "pub static SUPPORT_TRAILING: &[usize] = &[",
        &support_trailing,
        "];",
        writer
    )?;
    // main regex support index
    export_vec(
        "pub static SUPPORT_MAIN: &[usize] = &[",
        &support_main,
        "];",     
        writer
    )?;

    Ok(())
}

fn generate_alternative_token(
    alternatives: &HashMap<String, Vec<String>>, 
    symbol_lookup: &HashMap<String, u32>,
    writer: &mut impl Write) -> Result<(), anyhow::Error> 
{
    writeln!(writer, "{}", with_indent("pub static ALTERNATIVE_SYMBOL_TABLE: phf::Map<u32, &[u32]> = phf::phf_map!{", 1))?;

    for (symbol, values) in alternatives {
        export_alternative_pattern(symbol, values, symbol_lookup, writer)?;
    }
    writeln!(writer, "{}", with_indent("};", 1))?;

    Ok(())
}

fn export_rule_pattern(id: u32, name: &str, len: usize) -> proc_macro2::TokenStream {
    quote! { ScanPattern { id: #id, pattern: #name, len: #len }, }
}

fn export_rule_support<V: std::fmt::Display>(i: V, pattern: &str) -> String {
    format!("{i}, // {pattern}")
}

fn export_alternative_pattern(symbol: &str, alternatives: &[String], lookup: &HashMap<String, u32>, writer: &mut impl Write) -> Result<(), anyhow::Error> {
    let key = lookup.get(symbol).expect(&format!("Not found alternative key (`{symbol}`)"));

    writeln!(writer, "{}", with_indent(&format!("{key}u32 => &["), 2))?;

    for alt in alternatives {
        let id = lookup.get(alt).expect(&format!("Not found alternative value (`{alt}`)"));
        writeln!(writer, "{}", with_indent(&export_rule_support(*id, &alt), 3))?;
    }

    writeln!(writer, "{}", with_indent("],", 2))?;

    Ok(())
}

#[inline]
fn export_vec(preamble: &str, body: &[String], postamble: &str, writer: &mut impl Write) -> Result<(), anyhow::Error> {
    writeln!(writer, "{}", with_indent(preamble, 1))?;

    for s in body {
        writeln!(writer, "{}", with_indent(&s, 2))?;
    }

    writeln!(writer, "{}", with_indent(postamble, 1))?;

    Ok(())
}