use crate::SyntaxKind;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug, derive_builder::Builder)]
pub struct ParsingRuleSet {
    lookahead_translation: fn(kind_id: u32, state: usize) -> Option<&'static Transition>,
    goto_translation: fn(kind_id: u32, state: usize) -> Option<&'static usize>,
    accept_transition: fn() -> Option<&'static Transition>,
    symbol_lookup: fn(id: u32) -> &'static crate::SyntaxKind,
    alternative_symbol_lookup: fn(parent_kind_id: u32, child_kind_id: u32) -> Option<&'static crate::SyntaxKind>,
    candidate_symbols: fn(state: usize) -> Vec<&'static SyntaxKind>,
    full_emit_region: super::EmitRegin,
    statement_emit_region: super::EmitRegin,
    invalid_statement_emit_region: super::EmitRegin,
}

impl ParsingRuleSet {
    pub fn next_lookahead_state(&self, kind_id: u32, state: usize) -> Option<&'static Transition> {
        (self.lookahead_translation)(kind_id, state)
    }

    pub fn next_goto_state(&self, kind_id: u32, state: usize) -> Option<&'static usize> {
        (self.goto_translation)(kind_id, state)
    }

    pub fn accept_state(&self, state: usize) -> Option<&'static Transition> {
        let accept = (self.accept_transition)();

        match accept {
            Some(Transition::Accept { last_state, .. }) if *last_state == state => {
                accept
            }
            _ => None,
        }
    }

    pub fn from_kind_id(&self, id: u32) -> SyntaxKind {
        (self.symbol_lookup)(id).clone()
    }

    pub fn from_alt_symbol(&self, parent_kind: SyntaxKind, child_kind: SyntaxKind) -> Option<&'static SyntaxKind> {
        (self.alternative_symbol_lookup)(parent_kind.id, child_kind.id)
    }

    pub fn candidate_terminal_symbols(&self, state: usize) -> Vec<&'static SyntaxKind> {
        (self.candidate_symbols)(state)
    }

    pub fn statement_emit_config(&self) -> EmitConfig {
        EmitConfig{
            from_symbol: self.from_kind_id(self.statement_emit_region.start_item_id),
            to_symbol: self.from_kind_id(self.statement_emit_region.end_item_id),
        }
    }

    pub fn invalid_statement_emit_config(&self) -> EmitConfig {
        EmitConfig{
            from_symbol: self.from_kind_id(self.invalid_statement_emit_region.start_item_id),
            to_symbol: self.from_kind_id(self.invalid_statement_emit_region.end_item_id),
        }
    }

    pub fn full_emit_config(&self) -> EmitConfig {
        EmitConfig{
            from_symbol: self.from_kind_id(self.full_emit_region.start_item_id),
            to_symbol: self.from_kind_id(self.full_emit_region.end_item_id),
        }
    }
}

impl Default for ParsingRuleSet {
    fn default() -> Self {
        Self { 
            lookahead_translation: |_kind_id, _state| None,
            goto_translation: |_kind_id, _state| None,
            accept_transition: || None,
            symbol_lookup: crate::core::engine_core::scanner_engine::default_symbol_lookup,
            alternative_symbol_lookup: |_parent_kind_id, _child_kind_id| None,
            candidate_symbols: |_state| vec![],
            statement_emit_region: super::EmitRegin::default(),
            invalid_statement_emit_region: super::EmitRegin::default(),
            full_emit_region: super::EmitRegin::default(),
        }
    }
}

pub enum Transition {
    Shift { next_state: usize },
    Reduce{ pop_count: usize, lhs: u32 },
    Accept{ last_state: usize, lhs: u32 },
}

pub struct EmitConfig {
    pub from_symbol: SyntaxKind,
    pub to_symbol: SyntaxKind,
}