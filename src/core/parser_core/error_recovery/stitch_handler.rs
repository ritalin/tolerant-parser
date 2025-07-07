use std::collections::HashSet;
use crate::core::engine_core::parser_engine::{ParsingRuleSet, Transition};
use crate::core::scanner_core::Token;
use super::{RecoveryEvent, RecoveryReport, RecoveryEventPayload};

pub struct StitchRecoveryHandler {
    engine: ParsingRuleSet,
    terminate_kinds: HashSet<u32>,
}

impl StitchRecoveryHandler {
    pub fn new(engine: ParsingRuleSet) -> Self {
        let terminate_kinds = [
            engine.statement_emit_config().to_symbol.id,
            engine.full_emit_config().to_symbol.id
        ].into_iter().collect::<HashSet<u32>>();

        Self { engine, terminate_kinds }
    }

    pub fn try_recovery<'a>(&self, mut report: RecoveryReport, lookaheads: impl Iterator<Item = &'a Token>) -> Option<RecoveryReport> {
        let mut lookaheads = lookaheads.peekable();

        while let (Some(lookahead), Some(last_state)) = (lookaheads.peek(), report.state_stack.peek_state().cloned()) {
            let kind = lookahead.main.kind;
            if self.terminate_kinds.contains(&kind.id) { break }
        
            match self.engine.next_lookahead_state(kind.id, last_state) {
                Some(Transition::Shift { next_state }) => {
                    lookaheads.next();
                    report.state_stack.push_state(*next_state);
                    report.push_event(kind.id, 
                        RecoveryEvent::Stitch(RecoveryEventPayload::Shift { 
                            kind,
                            state: last_state, 
                            next_state: *next_state
                        })
                    );

                    // Contributed, but not reduce yet...
                    report.stitch_score += 1;
                }
                Some(Transition::Reduce { pop_count, lhs }) if *pop_count == 0 => {
                    let Some(goto_state) = report.state_stack.pop_n_state(*pop_count) else { continue };
                    let Some(next_state) = self.engine.next_goto_state(*lhs, *goto_state) else { continue };

                    report.state_stack.push_state(*next_state);
                    report.push_event(kind.id, 
                        RecoveryEvent::Stitch(RecoveryEventPayload::Reduce { 
                            kind: self.engine.from_kind_id(*lhs),
                            state: last_state, 
                            next_state: *next_state, 
                            pop_count: *pop_count
                        })
                    );

                    // it does not contributed transition.
                }
                Some(Transition::Reduce { pop_count, lhs }) => {
                    let Some(goto_state) = report.state_stack.pop_n_state(*pop_count) else { continue };
                    let Some(next_state) = self.engine.next_goto_state(*lhs, *goto_state) else { continue };

                    report.state_stack.push_state(*next_state);
                    report.push_event(kind.id, 
                        RecoveryEvent::Stitch(RecoveryEventPayload::Reduce { 
                            kind: self.engine.from_kind_id(*lhs),
                            state: last_state, 
                            next_state: *next_state, 
                            pop_count: *pop_count
                        })
                    );
                    
                    // Contributed reduce transition
                    report.stitch_score += 1;
                    break;
                }
                Some(Transition::Accept { last_state, lhs }) => {
                    report.push_event(kind.id, 
                        RecoveryEvent::Stitch(RecoveryEventPayload::Accept {
                            kind: self.engine.from_kind_id(*lhs),
                            last_state: *last_state
                        })
                    ); 

                    // Contributed accept transition
                    report.stitch_score += 1;
                    break;
                }
                None => {
                    return None;
                }
            }
        }

        Some(report)
    }
}