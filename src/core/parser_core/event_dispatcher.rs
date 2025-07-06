use std::collections::VecDeque;
use engine_core::{parser_engine::{ParsingRuleSet, Transition}, SyntaxKind};
use crate::{error_recovery::RecoveryEventPayload, parser::{ParseMode, RecoveryEvent}, state_stack::StateStack};

pub struct ParseEventDispatcher {
    mode: ParseMode,
    state_stack: StateStack,
    event_queue: VecDeque<ParseEvent>,
    engine: ParsingRuleSet,
}

impl ParseEventDispatcher {
    pub fn new(initial_state: usize, mode: ParseMode, engine: ParsingRuleSet) -> Self {
        Self {
            mode,
            state_stack: StateStack::new(initial_state),
            event_queue: VecDeque::new(),
            engine,
        }
    }

    pub fn next(&mut self, lookahead_kind: Option<SyntaxKind>) -> Result<ParseEvent, ParseEventError> {
        if ! self.event_queue.is_empty() {
            return Ok(self.event_queue.pop_front().unwrap());
        }

        // peek event
        let mut event = self.next_internal(lookahead_kind)?;
        let emit_config = self.engine.statement_emit_config();

        if self.mode == ParseMode::ByStatement {
            let initial_state = self.state_stack.initial_state();
            match event.kind() {
                kind if kind == emit_config.to_symbol => {
                    // additional emit event
                    self.event_queue.push_back(ParseEvent::Emit { kind: emit_config.from_symbol, edit_state: initial_state });
                }
                kind => {
                    let full_emit_config = self.engine.full_emit_config();
                    if kind == full_emit_config.to_symbol {
                        if let Some(top_state) = self.state_stack.peek_state() {
                            if *top_state != self.state_stack.initial_state() {
                                // A previous statement has not emitted
                                self.event_queue.push_back(event);
                                // In advance, It dispatches emit event
                                event = ParseEvent::PatchEmit { kind: emit_config.from_symbol, edit_state: initial_state };
                            }
                        }

                        // additional emit event
                        self.event_queue.push_back(ParseEvent::Emit { kind: emit_config.from_symbol, edit_state: initial_state });
                        // additional accept event
                        self.event_queue.push_back(ParseEvent::Accept { kind: full_emit_config.from_symbol, last_state: initial_state, edit_state: initial_state });
                    }
                }
            }
        }

        Ok(event)
    }

    fn next_internal(&mut self, lookahead_kind: Option<SyntaxKind>) -> Result<ParseEvent, ParseEventError> {
        let Some(state) = self.state_stack.peek_state().cloned() else {
            return Err(ParseEventError::NoMoreState{ context: "Shift".into() });
        };

        let Some(lookahead_kind) = lookahead_kind else {
            return match self.engine.accept_state(state) {
                Some(Transition::Accept { last_state, lhs }) => {
                    self.state_stack.pop_all();
                    let last_kind = self.engine.from_kind_id(*lhs);
                    Ok(ParseEvent::Accept { kind: last_kind, last_state: *last_state, edit_state: 0 })
                }
                _ => {
                    Err(ParseEventError::NotAccept)
                }
            };
        };

        match self.engine.next_lookahead_state(lookahead_kind.id, state) {
            Some(Transition::Shift { next_state }) => {
                self.state_stack.push_state(*next_state);
                let edit_state = self.state_stack.mark_checkpoint(state, true);

                Ok(ParseEvent::Shift { kind: lookahead_kind, current_state: state, next_state: *next_state, edit_state })
            }
            Some(Transition::Reduce { pop_count, lhs: goto_kind_id }) => {
                let Some(peek_state) = self.state_stack.pop_n_state(*pop_count) else {
                    return Err(ParseEventError::NoMoreState{ context: "Reduce".into() });
                };

                let lhs_kind = self.engine.from_kind_id(*goto_kind_id);
                let Some(goto_state) = self.engine.next_goto_state(*goto_kind_id, *peek_state) else {
                    return Err(ParseEventError::NoGotoCandidate { state: *peek_state, lhs: lhs_kind.text.into() })
                };
                self.state_stack.push_state(*goto_state);
                let edit_state = self.state_stack
                    .resolve_checkpoint(*pop_count)
                    .unwrap_or_else(|| self.state_stack.mark_checkpoint(state, false))
                ;

                Ok(ParseEvent::Reduce { kind: lhs_kind, current_state: state, next_state: *goto_state, pop_count: *pop_count, edit_state })
            }
            Some(Transition::Accept { last_state, lhs }) => {
                let last_kind = self.engine.from_kind_id(*lhs);
                Ok(ParseEvent::Accept { kind: last_kind, last_state: *last_state, edit_state: 0 })
            }
            None if lookahead_kind == self.engine.full_emit_config().to_symbol => {
                // fall back to handle EOF 
                let initial_state = self.state_stack.initial_state();
                let state = self.state_stack.peek_state().cloned().unwrap_or(initial_state);
                return Ok(ParseEvent::Shift { kind: lookahead_kind, current_state: state, next_state: initial_state, edit_state: initial_state });
            }
            None => {
                return Err(ParseEventError::RequestRecovery);
            }
        }
    }

    pub fn post_recovery_event(&mut self, events: &[RecoveryEvent]) {
        let initial_state = self.state_stack.initial_state();

        for recover in events {
            match recover {
                RecoveryEvent::PatchDelete { kind, state } => {
                    self.event_queue.push_back(
                        ParseEvent::PatchDrop { kind: *kind, current_state: *state, next_state: *state, edit_state: *state }
                    );
                }
                RecoveryEvent::PatchShift(RecoveryEventPayload::Shift { kind, state, next_state }) => {
                    self.state_stack.push_state(*next_state);
                    let edit_state = self.state_stack.mark_checkpoint(*state, true);
                    
                    self.event_queue.push_back(
                        ParseEvent::PatchShift { kind: *kind, current_state: *state, next_state: *next_state, edit_state }
                    );
                }
                RecoveryEvent::PatchShift(RecoveryEventPayload::Reduce { kind, state, next_state, pop_count }) => {
                    self.state_stack.pop_n_state(*pop_count);
                    self.state_stack.push_state(*next_state);

                    let edit_state = self.state_stack
                        .resolve_checkpoint(*pop_count)
                        .unwrap_or_else(|| self.state_stack.mark_checkpoint(*state, false))
                    ;
                    
                    self.event_queue.push_back(
                        ParseEvent::PatchReduce { kind: *kind, current_state: *state, next_state: *next_state, edit_state, pop_count: *pop_count }
                    );
                }
                RecoveryEvent::PatchShift(RecoveryEventPayload::Accept { .. }) => {
                    // In recovery patch pthase, Accept event does not fire.
                    continue;
                }
                RecoveryEvent::Stitch(RecoveryEventPayload::Shift { kind, state, next_state }) => {
                    self.state_stack.push_state(*next_state);
                    let edit_state = self.state_stack.mark_checkpoint(*state, true);

                    self.event_queue.push_back(
                        ParseEvent::Shift { kind: *kind, current_state: *state, next_state: *next_state, edit_state }
                    );
                }
                RecoveryEvent::Stitch(RecoveryEventPayload::Reduce { kind, state, next_state, pop_count }) => {
                    self.state_stack.pop_n_state(*pop_count);
                    self.state_stack.push_state(*next_state);

                    let edit_state = self.state_stack
                        .resolve_checkpoint(*pop_count)
                        .unwrap_or_else(|| self.state_stack.mark_checkpoint(*state, false))
                    ;
                    
                    self.event_queue.push_back(
                        ParseEvent::Reduce { kind: *kind, current_state: *state, next_state: *next_state, pop_count: *pop_count, edit_state }
                    );
                }
                RecoveryEvent::Stitch(RecoveryEventPayload::Accept { kind, last_state }) => {
                    self.event_queue.push_back(
                        ParseEvent::Accept { kind: *kind, last_state: *last_state, edit_state: initial_state }
                    );
                }
                RecoveryEvent::Invalid { kind, need_emit } => {
                    self.event_queue.push_back({
                        // peek top state
                        let state = self.state_stack.peek_state().cloned().unwrap_or(initial_state);
                        ParseEvent::Invalid { kind: *kind, current_state: state, edit_state: initial_state }
                    });

                    if *need_emit {
                        // post emit event
                        match self.mode {
                            ParseMode::ByStatement => {
                                let statement_emit = self.engine.statement_emit_config();
                                self.event_queue.push_back(
                                    ParseEvent::Emit { kind: statement_emit.from_symbol, edit_state: initial_state }
                                );
                            }
                            ParseMode::Full => {
                                // ParseMode::Full needs to rewind state stack
                                let statement_emit = self.engine.invalid_statement_emit_config();
                                let mut pop_count = 0;
                                while let Some(state) = self.state_stack.peek_state().cloned() {
                                    match self.engine.next_lookahead_state(statement_emit.to_symbol.id, state) {
                                        Some(Transition::Shift { next_state }) => {
                                            self.state_stack.push_state(*next_state);
                                            break;
                                        }
                                        _ => {}
                                    }
                                    
                                    self.state_stack.pop_n_state(1);
                                    pop_count += 1;
                                }

                                self.event_queue.push_back(
                                    ParseEvent::InvalidEmit { kind: statement_emit.from_symbol, edit_state: initial_state, pop_count: pop_count }
                                );
                            }
                        }
                    }
                }
            };
        }
    }

    pub fn has_next(&self) -> bool {
        ! self.event_queue.is_empty()
    }

    pub fn flush_state(&mut self) {
        self.state_stack.reset();
    }

    pub fn borrow_stack(&self) -> &StateStack {
        &self.state_stack
    }

    pub fn state_values(&self) -> Vec<usize> {
        self.state_stack.state_values()
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum ParseEvent {
    Shift { 
        kind: SyntaxKind, 
        /// transition before state
        current_state: usize, 
        /// transition after state
        next_state: usize, 
        /// edit state for incremental parsing
        edit_state: usize,
    },
    Reduce{ 
        kind: SyntaxKind, 
        /// transition before state
        current_state: usize, 
        /// transition after state
        next_state: usize, 
        /// count for popped from state stack
        pop_count: usize, 
        /// edit state for incremental parsing
        edit_state: usize,
    },
    Emit {
        kind: SyntaxKind, 
        /// edit state for incremental parsing
        edit_state: usize 
    },
    Accept{ 
        kind: SyntaxKind, 
        /// final state
        last_state: usize,
        /// edit state for incremental parsing
        edit_state: usize,
    },
    PatchDrop {
        kind: SyntaxKind, 
        /// transition before state
        current_state: usize, 
        /// transition after state
        next_state: usize, 
        /// edit state for incremental parsing
        edit_state: usize,
    },
    PatchShift { 
        kind: SyntaxKind, 
        /// transition before state
        current_state: usize, 
        /// transition after state
        next_state: usize, 
        /// edit state for incremental parsing
        edit_state: usize,
    },
    PatchReduce{ 
        kind: SyntaxKind, 
        /// transition before state
        current_state: usize, 
        /// transition after state
        next_state: usize, 
        /// count for popped from state stack
        pop_count: usize, 
        /// edit state for incremental parsing
        edit_state: usize,
    },
    PatchEmit{
        kind: SyntaxKind, 
        /// edit state for incremental parsing
        edit_state: usize,
    },
    Invalid{
        kind: SyntaxKind, 
        /// transition before state
        current_state: usize, 
        /// edit state for incremental parsing
        edit_state: usize,
    },
    InvalidEmit{
        kind: SyntaxKind, 
        /// edit state for incremental parsing
        edit_state: usize,
        /// count for popped from state stack
        pop_count: usize, 
    }
}

impl ParseEvent {
    pub fn kind(&self) -> SyntaxKind {
        match self {
            ParseEvent::Shift { kind, .. } => *kind,
            ParseEvent::Reduce { kind, .. } => *kind,
            ParseEvent::Emit { kind, .. } => *kind,
            ParseEvent::Accept { kind, .. } => *kind,
            ParseEvent::PatchDrop { kind, .. } => *kind,
            ParseEvent::PatchShift { kind, .. } => *kind,
            ParseEvent::PatchReduce { kind, .. } => *kind,
            ParseEvent::PatchEmit { kind, .. } => *kind,
            ParseEvent::Invalid { kind, .. } => *kind,
            ParseEvent::InvalidEmit { kind, .. } => *kind,
        }
    }
}

#[derive(PartialEq, Debug, thiserror::Error)]
pub enum ParseEventError {
    /// no more entry in state stack
    #[error("no more entry in state stack (context: {context})")]
    NoMoreState { context: String },
    /// no more entry in state stack
    #[error("no more entry in goto candidate (state: {state}, lhs: {lhs})")]
    NoGotoCandidate {state: usize, lhs: String},
    /// request to recover parsing state
    #[error("request to recover parsing state")]
    RequestRecovery,
    /// unmatch accept state
    #[error("unmatch accept state")]
    NotAccept,

}
