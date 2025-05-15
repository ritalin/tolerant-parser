

#[cfg(not(engine_ungenerated))]
mod generated {
    use engine_core::{parser_engine::Transition, scanner_engine::AcceptableRegexSet};
    
    include!("_generated/symbol_set.rs");
    include!("_generated/scan_rule.rs");
    include!("_generated/parse_rule.rs");

    pub fn get_lexme_pattern(prefix: char) -> Option<&'static [engine_core::scanner_engine::ScanPattern]> {
        scan_rule_map::LEXME_SCAN_RULE.get(&prefix).cloned()
    }

    pub fn get_regex_pattern(index: usize) -> Option<&'static engine_core::scanner_engine::ScanPattern> {
        scan_rule_map::REGEX_SCAN_RULE.get(index)
    }

    pub fn get_symbol(symbol_id: u32) -> &'static engine_core::SyntaxKind {
        syntax_map::SYNTAX_KIND_MAP.get(&symbol_id).cloned().unwrap_or(&syntax_kind::r#ILLEGAL)
    }

    pub fn get_acceptable_regex_indexes(regex_set: &AcceptableRegexSet) -> Option<&'static [usize]> {
        match regex_set {
            AcceptableRegexSet::Leading => Some(scan_rule_map::SUPPORT_LEADING),
            AcceptableRegexSet::Main => Some(scan_rule_map::SUPPORT_MAIN),
            AcceptableRegexSet::Trailing => Some(scan_rule_map::SUPPORT_TRAILING),
        }
    }

    pub fn next_lookahead_state(kind_id: u32, state: usize) -> Option<&'static Transition> {
        lookahead_transition::TABLES[state].get(&kind_id)
    }

    pub fn next_goto_state(lhs_kind_id: u32, state: usize) -> Option<&'static usize> {
        let Some(ref goto_table) = goto_transition::TABLES[state] else {
            return None;
        };
        
        goto_table.get(&lhs_kind_id)
    }

    pub fn get_accept_state() -> Option<&'static Transition> {
        lookahead_transition::ACCEPT.as_ref()
    }
}

#[cfg(not(engine_ungenerated))]
pub fn create() -> Result<engine_core::Engine, engine_core::EngineError> {
    use generated::syntax_kind;

    Ok(engine_core::Engine {
        scanning_rules: engine_core::scanner_engine::ScanningRuleSet::new(
            generated::get_lexme_pattern,
            generated::get_regex_pattern,
            generated::get_acceptable_regex_indexes,
            generated::get_symbol,
            syntax_kind::r#EOF.id,
        ),
        parsing_rules: engine_core::parser_engine::ParsingRuleSet::new(
            generated::next_lookahead_state, 
            generated::next_goto_state, 
            generated::get_accept_state,
            generated::get_symbol,
            syntax_kind::r#EOF.id,
        ),
    })
}
#[cfg(not(engine_ungenerated))]
pub use generated::syntax_kind;

#[cfg(engine_ungenerated)]
pub fn create() -> Result<engine_core::Engine, engine_core::EngineError> {
    Ok(engine_core::Engine::default())
}
#[cfg(engine_ungenerated)]
pub use engine_core::default_syntax_kind as syntax_kind;
