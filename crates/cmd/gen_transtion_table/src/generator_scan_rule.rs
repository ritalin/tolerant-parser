use std::{collections::{BTreeMap, HashMap}, path::PathBuf};
use std::io::{Write, BufWriter};
use tolerant_parser_sdk::core::engine_core::scanner_engine::CaseSensitivity;
use tolerant_parser_sdk::support::grammar_types::{scan_rule::{AltPattern, GrammarScanRule, RegexGrammarScanRule}, symbol::GrammarSymbol, SymbolType};
use quote::quote;

use crate::export_support::{tokens_to_string, with_indent};

pub fn generate(
    rules: &GrammarScanRule, 
    symbols: &[GrammarSymbol], 
    symbol_lookup: &HashMap<String, u32>, 
    output_dir: PathBuf) -> Result<(), anyhow::Error> 
{
    let output_file = crate::storage_support::make_output_file(output_dir, "scan_rule.rs")?;
    let mut writer = BufWriter::new(output_file);

    writeln!(writer, "mod scan_rule_map {{")?;
    writeln!(writer, "{}", with_indent("use tolerant_parser_sdk::core::engine_core::scanner_engine::{ScanPattern, CaseSensitivity};", 1))?;

    generate_lexme_scan_rule(&rules.lexme, collect_keywords(symbols), symbol_lookup, &rules.ignore_case_override, &mut writer)?;
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
    _ignore_case_override: &HashMap<String, bool>,
    writer: &mut impl Write) -> Result<(), anyhow::Error> 
{
    let mut scan_rules = BTreeMap::<char, Vec<String>>::new();
    
    // Prepare keyword prefix map
    for symbol in keywords {
        let case_sensitive = None;
        let rule = tokens_to_string(export_rule_pattern(symbol.id, &symbol.name, symbol.name.len(), case_sensitive), 3);
        let (_, prefix) = symbol.name.char_indices().next().unwrap();

        scan_rules.entry(prefix.to_ascii_lowercase())
            .and_modify(|xs| xs.push(rule.clone()))
            .or_insert_with(|| vec![rule])
        ;
    }

    for (name, patterns) in lexme {
        let id = *symbol_lookup.get(name).unwrap();

        for pattern in patterns {
            let case_sensitive = None;
            let rule = tokens_to_string(export_rule_pattern(id, &pattern, pattern.len(), case_sensitive), 3);
            let (_, prefix) = pattern.char_indices().next().unwrap();

            scan_rules.entry(prefix.to_ascii_lowercase())
                .and_modify(|xs| xs.push(rule.clone()))
                .or_insert_with(|| vec![rule])
            ;
        }
    }

    writeln!(writer, "{}", with_indent("#[cfg(engine_ungenerated)]", 1))?;
    writeln!(writer, "{}", with_indent("pub static LEXME_SCAN_RULE: phf::Map<char, &'static [ScanPattern]> = phf::phf_map!{};", 1))?;
    writeln!(writer, "{}", with_indent("#[cfg(not(engine_ungenerated))]", 1))?;
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
            let rule = tokens_to_string(export_rule_pattern(id, &pattern.pattern, pattern.pattern.len(), Some(CaseSensitivity::Sensitive)), 1);
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
    alternatives: &HashMap<String, Vec<AltPattern>>, 
    symbol_lookup: &HashMap<String, u32>,
    writer: &mut impl Write) -> Result<(), anyhow::Error> 
{
    writeln!(writer, "{}", with_indent("pub static ALTERNATIVE_SYMBOL_TABLE: phf::Map<u64, u32> = phf::phf_map!{", 1))?;

    for (alt, pair) in alternatives {
        export_alternative_pattern(alt, pair, symbol_lookup, writer)?;
    }
    writeln!(writer, "{}", with_indent("};", 1))?;

    Ok(())
}

fn export_rule_pattern(id: u32, name: &str, len: usize, case_sensitive: Option<CaseSensitivity>) -> proc_macro2::TokenStream {
    let case_sensitive = match case_sensitive {
        Some(CaseSensitivity::Insensitive) => quote!(Some(CaseSensitivity::Insensitive)),
        Some(CaseSensitivity::Sensitive) => quote!(Some(CaseSensitivity::Sensitive)),
        None => quote!(None)
    };

    quote! { ScanPattern { id: #id, pattern: #name, len: #len, case_sensitive: #case_sensitive }, }
}

fn export_rule_support<V: std::fmt::Display>(i: V, pattern: &str) -> String {
    format!("{i}, // {pattern}")
}

fn export_alternative_pattern(alt_symbol: &str, pairs: &[AltPattern], lookup: &HashMap<String, u32>, writer: &mut impl Write) -> Result<(), anyhow::Error> {
    let alt_id = lookup.get(alt_symbol).expect(&format!("Not found alternative key (`{alt_symbol}`)"));

    for AltPattern{ parent, child } in pairs {
        let parent_id = lookup.get(parent).expect(&format!("Not found alternative parent key (`{parent}`)"));
        let child_id = lookup.get(child).expect(&format!("Not found alternative child key (`{child}`)"));
        let key = ((*parent_id as u64) << 32) + (*child_id as u64);
        let comment = format!("{parent} |> {child} => {parent} |> {alt_symbol}");

        writeln!(writer, "{}", with_indent(&export_rule_support(format!("{key}u64 => {alt_id}u32"), &comment), 2))?;
    }

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