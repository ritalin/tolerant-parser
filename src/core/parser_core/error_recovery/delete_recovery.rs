use crate::core::engine_core::parser_engine::ParsingRuleSet;
use crate::core::scanner_core::Token;
use crate::core::parser_core::state_stack::StateStack;

use super::{RecoveryEvent, RecoveryPenalty, RecoveryReport};



pub struct DeleteErrorRecovery {
    state_stack: StateStack,
    penalty: RecoveryPenalty,
    engine: ParsingRuleSet,
}

impl DeleteErrorRecovery {
    #[cfg(feature = "test_support")]
    #[doc(hidden)]
    pub fn new(state_histories: &[usize], penalty: RecoveryPenalty, engine: ParsingRuleSet) -> Self {
        Self::new_with_stack(super::make_stack(state_histories), penalty, engine)
    }

    pub(crate) fn new_with_stack(state_stack: StateStack, penalty: RecoveryPenalty, engine: ParsingRuleSet) -> Self {
        Self {
            state_stack,
            penalty,
            engine,
        }
    }

    pub fn handle<'a, I>(&mut self, lookaheads: I) -> Option<RecoveryReport> 
    where I: Iterator<Item = &'a Token> 
    {
        if self.penalty.delete_slot == 0 { 
            return None; 
        }
        let Some(top_state) = self.state_stack.peek_state() else {
            return None;
        };

        let mut lookaheads = lookaheads.peekable();

        // drop lookahead
        let mut report = RecoveryReport::new_with_stack(self.state_stack.clone());

        while let (Some(lookahead), Some(next_lookahad))= (lookaheads.next(), lookaheads.peek()) {
            self.penalty.delete_slot -= 1;
            report.patch_score += 1;
            report.push_event(lookahead.main.kind.id, RecoveryEvent::PatchDelete { 
                kind: lookahead.main.kind,
                state: *top_state, 
            });

            match self.engine.next_lookahead_state(next_lookahad.main.kind.id, *top_state) {
                Some(_) => return Some(report),
                None => {}
            }

            if self.penalty.delete_slot == 0 {
                break;
            }
        }

        None
    }

    pub fn left_slot(&self) -> usize {
        self.penalty.delete_slot
    }
}
