use crate::SyntaxKind;

pub struct ParsingRuleSet {
    translation_table: fn(kind_id: u32, state: usize) -> Option<Transition>,
}

impl ParsingRuleSet {
    pub fn translate_state(&self, kind_id: u32, state: usize) -> Option<Transition> {
        todo!()
    }

    pub fn goto_state(&self, kind_id: u32, state: usize) -> Option<usize> {
        todo!()
    }
}

impl Default for ParsingRuleSet {
    fn default() -> Self {
        Self { translation_table: default_next_translation }
    }
}

fn default_next_translation(_kind_id: u32, _state: usize) -> Option<Transition> {
    None
}

pub enum Transition {
    Shift { next_state: usize },
    Reduce{ pop_count: usize, lhs: u32 },
    Accept{ last_state: usize },
}
