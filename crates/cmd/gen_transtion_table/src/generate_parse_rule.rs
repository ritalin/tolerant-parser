use std::{collections::HashMap, io::{BufWriter, Write}, path::PathBuf};
use grammar_types_core::parse_rule::RuleId;
use crate::{export_support::{tokens_to_string, with_indent}, translation_table::GrammarSymbolRef};
use quote::quote;

pub fn generate(
    table: &lalry::LR1ParseTable<GrammarSymbolRef, String, RuleId>,
    symbol_lookup: &HashMap<String, u32>,
    start_synbol: &str,
    output_dir: PathBuf) -> Result<(), anyhow::Error>
{
    let output_file = crate::storage_support::make_output_file(output_dir, "parse_rule.rs")?;
    let mut writer = BufWriter::new(output_file);

    generate_lookahead_state(&table.states, &symbol_lookup, start_synbol, 1, &mut writer)?;
    generate_goto_state(&table.states, &symbol_lookup, 1, &mut writer)?;

    Ok(())
}

fn generate_lookahead_state(
    parse_states: &[lalry::LR1State<GrammarSymbolRef, String, RuleId>], 
    symbol_lookup: &HashMap<String, u32>, 
    start_symbol: &str,
    indent: usize,
    writer: &mut impl Write) -> Result<(), anyhow::Error> 
{
    writeln!(writer, "mod lookahead_transition {{")?;
    writeln!(writer, "{}", with_indent("use tolerant_parser_sdk::core::engine_core::parser_engine::Transition;", indent))?;
    writeln!(writer, "{}", with_indent("#[cfg(engine_ungenerated)]", indent))?;
    writeln!(writer, "{}", with_indent("pub static TABLES: &[phf::Map<u32, Transition>] = &[];", indent))?;
    writeln!(writer, "{}", with_indent("#[cfg(not(engine_ungenerated))]", indent))?;
    writeln!(writer, "{}", with_indent("pub static TABLES: &[phf::Map<u32, Transition>] = &[", indent))?;

    for (i, state) in parse_states.iter().enumerate() {
        writeln!(writer, "{}", with_indent(&format!("// state: #{i}"), indent+1))?;
        writeln!(writer, "{}", with_indent("phf::phf_map! {", indent+1))?;

        for (la, action) in &state.lookahead {
            generate_lookahead_state_member(la, &action, symbol_lookup, indent+2, writer)?;
        }
        writeln!(writer, "{}", with_indent("},", 2))?;
    }

    writeln!(writer, "{}", with_indent("];", 1))?;

    generate_accept_state(parse_states, symbol_lookup.get(start_symbol), indent, writer)?;

    writeln!(writer, "}}")?;

    Ok(())
}

fn generate_lookahead_state_member(
    GrammarSymbolRef(la): &GrammarSymbolRef, 
    action: &lalry::LRAction<GrammarSymbolRef, String, RuleId>,
    symbol_lookup: &HashMap<String, u32>,
    indent: usize,
    writer: &mut impl Write) -> Result<(), anyhow::Error>
{
    let la_id = la.id;

    match action {
        lalry::LRAction::Reduce(lhs, rhs) => {
            let pop_count = rhs.syms.len();
            let lhs_id = symbol_lookup.get(*lhs).expect(&format!("Mismatch symbol id (symbol: {})", lhs));

            let rule = quote! {
                #la_id => Transition::Reduce { pop_count: #pop_count, lhs: #lhs_id },
            };
            write!(writer, "{}", tokens_to_string(rule, indent))?;
            writeln!(writer, " // LA: {}", la.name)?;
        }
        lalry::LRAction::Shift(next_state) => {
            let rule = quote! {
                #la_id => Transition::Shift { next_state: #next_state },
            };
            write!(writer, "{}", tokens_to_string(rule, indent))?;
            writeln!(writer, " // LA: {}", la.name)?;
        }
        lalry::LRAction::Accept => {}
    }

    Ok(())
}

fn generate_goto_state(
    parse_states: &[lalry::LR1State<GrammarSymbolRef, String, RuleId>], 
    symbol_lookup: &HashMap<String, u32>, 
    indent: usize,
    writer: &mut impl Write) -> Result<(), anyhow::Error> 
{
    writeln!(writer, "mod goto_transition {{")?;
    writeln!(writer, "{}", with_indent("#[cfg(engine_ungenerated)]", indent))?;
    writeln!(writer, "{}", with_indent("pub static TABLES: &[Option<phf::Map<u32, usize>>] = &[];", indent))?;
    writeln!(writer, "{}", with_indent("#[cfg(not(engine_ungenerated))]", indent))?;
    writeln!(writer, "{}", with_indent("pub static TABLES: &[Option<phf::Map<u32, usize>>] = &[", indent))?;

    for (i, state) in parse_states.iter().enumerate() {
        match state.goto.is_empty() {
            false => {
                writeln!(writer, "{}", with_indent(&format!("// state: #{i}"), indent+1))?;
                writeln!(writer, "{}", with_indent("Some(phf::phf_map! {", indent+1))?;

                for (lhs, next_state) in &state.goto {
                    let symbol_id = symbol_lookup.get(*lhs).expect(&format!("Mismatch goto lhs id (symbol: {}", lhs));
                    write!(writer, "{}", tokens_to_string(quote! { #symbol_id => #next_state, }, indent+2))?;
                    writeln!(writer, " // LHS: {}", lhs)?;
                }

                writeln!(writer, "{}", with_indent("}),", indent+1))?;
            }
            true => {
                writeln!(writer, "{}", with_indent(&format!("// state: #{i}"), indent+1))?;
                writeln!(writer, "{}", tokens_to_string(quote! { None, }, indent+1))?;
            }
        }

    }
    writeln!(writer, "{}", with_indent("];", indent))?;
    writeln!(writer, "}}")?;

    Ok(())
}

fn generate_accept_state(
    parse_states: &[lalry::LR1State<GrammarSymbolRef, String, RuleId>],
    start_symbol_id: Option<&u32>,
    indent: usize,
    writer: &mut impl Write) -> Result<(), anyhow::Error>
{
    let accept_state = parse_states.iter().enumerate()
        .find(|(_, state)| state.eof.is_some())
    ;

    let transition = match (accept_state, start_symbol_id) {
        (Some((state, _)), symbol_id) => {
            quote!{
                pub static ACCEPT: Option<Transition> = Some(Transition::Accept {
                    last_state: #state, 
                    lhs: #symbol_id
                });
            }
        }
        _ => {
            quote!{ pub static ACCEPT: Option<Transition> = None; }
        }
    };
    writeln!(writer, "{}", tokens_to_string(transition, indent))?;
    Ok(())
}
