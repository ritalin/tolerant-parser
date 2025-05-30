

#[cfg(not(engine_ungenerated))]
pub(crate) mod generated {
    use engine_core::{parser_engine::Transition, scanner_engine::AcceptableRegexSet, SyntaxKind};
    
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
        super::generated::syntax_map::SYNTAX_KIND_MAP.get(&symbol_id).cloned().unwrap_or(&super::syntax_kind::r#ILLEGAL)
    }

    pub fn get_acceptable_regex_indexes(regex_set: &AcceptableRegexSet) -> Option<&'static [usize]> {
        match regex_set {
            AcceptableRegexSet::Leading => Some(scan_rule_map::SUPPORT_LEADING),
            AcceptableRegexSet::Main => Some(scan_rule_map::SUPPORT_MAIN),
            AcceptableRegexSet::Trailing => Some(scan_rule_map::SUPPORT_TRAILING),
        }
    }

    pub fn get_candidate_symbols(state: usize) -> Vec<&'static SyntaxKind> {
        lookahead_transition::TABLES[state].keys()
        .map(|&id| get_symbol(id))
        .collect()
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

    pub fn get_alternative_symbol(parent_kind_id: u32, child_kind_id: u32) -> Option<&'static engine_core::SyntaxKind> {
        let key = ((parent_kind_id as u64) << 32) + (child_kind_id as u64);
        scan_rule_map::ALTERNATIVE_SYMBOL_TABLE.get(&key)
        .map(|id| get_symbol(*id))
    }
}

#[cfg(not(engine_ungenerated))]
pub fn create() -> Result<engine_core::Engine, engine_core::EngineError> {
    Ok(engine_core::Engine {
        scanning_rules: 
            builder::scan_rule_builder()
            .build()
            .map_err(|err| engine_core::EngineError::ScanningRuleCreateFailed(err.to_string()))?,
        parsing_rules: 
            builder::parse_rule_builder()
            .build()
            .map_err(|err| engine_core::EngineError::PrsingRuleCreateFailed(err.to_string()))?
    })
}
#[cfg(not(engine_ungenerated))]
pub mod builder {
    pub use super::generated::{
        get_lexme_pattern,
        get_regex_pattern,
        get_symbol,
        get_acceptable_regex_indexes,
        get_candidate_symbols,
        next_lookahead_state,
        next_goto_state,
        get_accept_state,
        get_alternative_symbol
    };

    pub fn scan_rule_builder() -> engine_core::scanner_engine::ScanningRuleSetBuilder {
        let mut builder = engine_core::scanner_engine::ScanningRuleSetBuilder::default();

        builder
            .lexme_rule(get_lexme_pattern)
            .regex_rule(get_regex_pattern, get_acceptable_regex_indexes)
            .symbol_lookup(get_symbol)
            .eof_id(super::syntax_kind::r#EOF.id)
            .invalid_id(super::syntax_kind::r#ILLEGAL.id)
        ;

        builder
    }

    pub fn parse_rule_builder() -> engine_core::parser_engine::ParsingRuleSetBuilder {
        let mut builder = engine_core::parser_engine::ParsingRuleSetBuilder::default();

        builder
            .lookahead_translation(next_lookahead_state)
            .goto_translation(next_goto_state)
            .accept_transition(get_accept_state)
            .alternative_symbol_lookup(get_alternative_symbol)
            .symbol_lookup(get_symbol)
            .candidate_symbols(get_candidate_symbols)
            .full_emit_config(super::syntax_kind::r#input.id, super::syntax_kind::r#EOF.id)
            .statement_emit_config(super::syntax_kind::r#ecmd.id, super::syntax_kind::r#SEMI.id)
            .invalid_statement_emit_config(super::syntax_kind::r#cmdx.id, super::syntax_kind::r#SEMI.id)
        ;
        builder
    }
}

#[cfg(not(engine_ungenerated))]
pub use generated::syntax_kind;

#[cfg(engine_ungenerated)]
pub fn create() -> Result<engine_core::Engine, engine_core::EngineError> {
    Ok(engine_core::Engine::default())
}
#[cfg(engine_ungenerated)]
pub use engine_core::default_syntax_kind as syntax_kind;
