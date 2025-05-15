use crate::SyntaxKind;

pub struct ParsingRuleSet {
    lookahead_translation: fn(kind_id: u32, state: usize) -> Option<&'static Transition>,
    goto_translation: fn(kind_id: u32, state: usize) -> Option<&'static usize>,
    accept_transition: fn() -> Option<&'static Transition>,
    symbol_lookup: fn(id: u32) -> &'static crate::SyntaxKind,
    eof_id: u32,
}

impl ParsingRuleSet {
    pub fn new(
        lookahead_translation: fn(kind_id: u32, state: usize) -> Option<&'static Transition>,
        goto_translation: fn(kind_id: u32, state: usize) -> Option<&'static usize>,
        accept_transition: fn() -> Option<&'static Transition>,
        symbol_lookup: fn(id: u32) -> &'static crate::SyntaxKind,
        eof_id: u32) -> Self 
    {
        Self {
            lookahead_translation,
            goto_translation,
            accept_transition,
            symbol_lookup,
            eof_id,
        }
    }

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

    pub fn eof(&self) -> SyntaxKind {
        self.from_kind_id(self.eof_id)
    }
}

impl Default for ParsingRuleSet {
    fn default() -> Self {
        Self { 
            lookahead_translation: default_next_lookahead_translation,
            goto_translation: default_next_goto_translation,
            accept_transition: default_accept_translation,
            symbol_lookup: crate::scanner_engine::default_symbol_lookup,
            eof_id: crate::default_syntax_kind::DEFAULT.id,
        }
    }
}

fn default_next_lookahead_translation(_kind_id: u32, _state: usize) -> Option<&'static Transition> {
    None
}

fn default_next_goto_translation(_kind_id: u32, _state: usize) -> Option<&'static usize> {
    None
}

fn default_accept_translation() -> Option<&'static Transition> {
    None
}

pub enum Transition {
    Shift { next_state: usize },
    Reduce{ pop_count: usize, lhs: u32 },
    Accept{ last_state: usize, lhs: u32 },
}
