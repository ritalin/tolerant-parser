use std::collections::VecDeque;
use engine_core::{parser_engine::{ParsingRuleSet, Transition}, SyntaxKind};
use crate::{error_recovery::RecoveryEventPayload, parser::RecoveryEvent, state_stack::StateStack};

pub struct ParseEventDispatcher {
    state_stack: StateStack,
    event_queue: VecDeque<ParseEvent>,
    engine: ParsingRuleSet,
}

impl ParseEventDispatcher {
    pub fn new(initial_state: usize, engine: ParsingRuleSet) -> Self {
        Self {
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
        let event = self.next_internal(lookahead_kind)?;

        if let Some(config) = self.engine.statement_emit_config() {
            let initial_state = self.state_stack.initial_state();
            match event.kind() {
                kind if kind == config.to_symbol => {
                    // additional emit event
                    self.event_queue.push_back(ParseEvent::Emit { kind: config.from_symbol, edit_state: initial_state });
                }
                kind => {
                    let full_emit_config = self.engine.full_emit_config();
                    if kind == full_emit_config.to_symbol {
                        // additional emit event
                        self.event_queue.push_back(ParseEvent::Emit { kind: config.from_symbol, edit_state: initial_state });
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
                let edit_state = self.state_stack.mark_checkpoint(state);

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
                    .unwrap_or_else(|| self.state_stack.mark_checkpoint(state))
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
                        ParseEvent::RecoverDrop { kind: *kind, current_state: *state, next_state: *state, edit_state: *state }
                    );
                }
                RecoveryEvent::PatchShift(RecoveryEventPayload::Shift { kind, state, next_state }) => {
                    self.state_stack.push_state(*next_state);
                    let edit_state = self.state_stack.mark_checkpoint(*state);
                    
                    self.event_queue.push_back(
                        ParseEvent::RecoverShift { kind: *kind, current_state: *state, next_state: *next_state, edit_state }
                    );
                }
                RecoveryEvent::PatchShift(RecoveryEventPayload::Reduce { kind, state, next_state, pop_count }) => {
                    self.state_stack.pop_n_state(*pop_count);
                    self.state_stack.push_state(*next_state);

                    let edit_state = self.state_stack
                        .resolve_checkpoint(*pop_count)
                        .unwrap_or_else(|| self.state_stack.mark_checkpoint(*state))
                    ;
                    
                    self.event_queue.push_back(
                        ParseEvent::RecoverReduce { kind: *kind, current_state: *state, next_state: *next_state, edit_state, pop_count: *pop_count }
                    );
                }
                RecoveryEvent::PatchShift(RecoveryEventPayload::Accept { .. }) => {
                    // In recovery patch pthase, Accept event does not fire.
                    continue;
                }
                RecoveryEvent::Stitch(RecoveryEventPayload::Shift { kind, state, next_state }) => {
                    self.state_stack.push_state(*next_state);
                    let edit_state = self.state_stack.mark_checkpoint(*state);

                    self.event_queue.push_back(
                        ParseEvent::Shift { kind: *kind, current_state: *state, next_state: *next_state, edit_state }
                    );
                }
                RecoveryEvent::Stitch(RecoveryEventPayload::Reduce { kind, state, next_state, pop_count }) => {
                    self.state_stack.pop_n_state(*pop_count);
                    self.state_stack.push_state(*next_state);

                    let edit_state = self.state_stack
                        .resolve_checkpoint(*pop_count)
                        .unwrap_or_else(|| self.state_stack.mark_checkpoint(*state))
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
                        if let Some(config) = self.engine.statement_emit_config() {
                            // post emit event
                            self.event_queue.push_back(
                                ParseEvent::Emit { kind: config.from_symbol, edit_state: initial_state }
                            );
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
    RecoverDrop {
        kind: SyntaxKind, 
        /// transition before state
        current_state: usize, 
        /// transition after state
        next_state: usize, 
        /// edit state for incremental parsing
        edit_state: usize,
    },
    RecoverShift { 
        kind: SyntaxKind, 
        /// transition before state
        current_state: usize, 
        /// transition after state
        next_state: usize, 
        /// edit state for incremental parsing
        edit_state: usize,
    },
    RecoverReduce{ 
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
    Invalid{
        kind: SyntaxKind, 
        /// transition before state
        current_state: usize, 
        /// edit state for incremental parsing
        edit_state: usize,
    }
}

impl ParseEvent {
    pub fn kind(&self) -> SyntaxKind {
        match self {
            ParseEvent::Shift { kind, .. } => *kind,
            ParseEvent::Reduce { kind, .. } => *kind,
            ParseEvent::Emit { kind, .. } => *kind,
            ParseEvent::Accept { kind, .. } => *kind,
            ParseEvent::RecoverDrop { kind, .. } => *kind,
            ParseEvent::RecoverShift { kind, .. } => *kind,
            ParseEvent::RecoverReduce { kind, .. } => *kind,
            ParseEvent::Invalid { kind, .. } => *kind,
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
