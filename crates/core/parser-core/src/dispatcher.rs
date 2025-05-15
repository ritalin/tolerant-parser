use cactus::Cactus;
use engine_core::{parser_engine::{ParsingRuleSet, Transition}, SyntaxKind};

pub struct ParseEventDispatcher {
    state_stack: StateStack,
    engine: ParsingRuleSet,
}

impl ParseEventDispatcher {
    pub fn new(initial_state: usize, engine: ParsingRuleSet) -> Self {
        Self {
            state_stack: StateStack::new(initial_state),
            engine,
        }
    }

    pub fn next(&mut self, lookahead_kind: Option<SyntaxKind>) -> Result<ParseEvent, crate::ParseError> {
        let Some(state) = self.state_stack.peek_state().cloned() else {
            return Err(crate::ParseError::NoMoreState{ context: "Shift".into() });
        };

        let Some(lookahead_kind) = lookahead_kind else {
            return match self.engine.accept_state(state) {
                Some(Transition::Accept { last_state, lhs }) => {
                    self.state_stack.pop_all();
                    let last_kind = self.engine.from_kind_id(*lhs);
                    Ok(ParseEvent::Accept { kind: last_kind, last_state: *last_state, edit_state: 0 })
                }
                _ => {
                    Err(crate::ParseError::NotAccept)
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
                    return Err(crate::ParseError::NoMoreState{ context: "Reduce".into() });
                };

                let lhs_kind = self.engine.from_kind_id(*goto_kind_id);
                let Some(goto_state) = self.engine.next_goto_state(*goto_kind_id, *peek_state) else {
                    return Err(crate::ParseError::NoGotoCandidate { state: *peek_state, lhs: lhs_kind.text.into() })
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
            None if lookahead_kind == self.engine.eof() => {
                // fall back to handle EOF 
                let state = self.state_stack.pop_n_state(1).cloned().unwrap_or(0);
                return Ok(ParseEvent::Shift { kind: lookahead_kind, current_state: state, next_state: 0, edit_state: 0 });
            }
            None => {
                return Err(crate::ParseError::RequestRecovery);
            }
        }
    }

    pub fn state_values(&self) -> Vec<usize> {
        self.state_stack.state_values()
    }
}

struct StateStack {
    initial_state: usize,
    stack: Cactus<usize>,
    checkpoint: Cactus<usize>,
}

impl StateStack {
    pub fn new(initial_state: usize) -> Self {
        Self { 
            initial_state,
            stack: Cactus::new().child(initial_state),
            checkpoint: Cactus::new(),
        }
    }

    pub fn peek_state(&self) -> Option<&usize> {
        self.stack.val()
    }

    pub fn push_state(&mut self, state: usize) {
        self.stack = self.stack.child(state);
    }

    pub fn pop_n_state(&mut self, mut pop_count: usize) -> Option<&usize> {
        while pop_count > 0 {
            let Some(parent) = self.stack.parent() else { break };
            self.stack = parent;
            pop_count -= 1;
        }

        assert!(pop_count == 0);

        self.peek_state()
    }

    pub fn pop_all(&mut self) {
        self.pop_n_state(self.stack.len());
    }

    pub fn reset(&mut self) {
        self.stack = Cactus::new().child(self.initial_state);
        self.checkpoint = Cactus::new();
    }

    pub fn state_values(&self) -> Vec<usize> {
        let mut values = vec![];

        let mut next_node = self.stack.clone();
        while let Some(v) = next_node.val() {
            values.push(*v);
            
            let Some(node) = next_node.parent() else {
                break
            };
            next_node = node;
        }

        values
    }

    pub fn mark_checkpoint(&mut self, state: usize) -> usize {
        self.checkpoint = self.checkpoint.child(state);
        state
    }
    pub fn resolve_checkpoint(&mut self, mut pop_count: usize) -> Option<usize> {
        if pop_count == 0 {
            return None;
        }

        while pop_count > 1 {
            self.checkpoint = self.checkpoint.parent().unwrap_or_default();
            pop_count -= 1;
        }
        self.checkpoint.val().cloned()

    }
}

#[derive(PartialEq, Debug)]
pub enum ParseEvent {
    Shift { 
        kind: SyntaxKind, 
        /// transition before state
        current_state: usize, 
        /// transition after state
        next_state: usize, 
        /// edit state for incremental parsing
        edit_state: usize 
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
        edit_state: usize 
    },
    Accept{ 
        kind: SyntaxKind, 
        /// final state
        last_state: usize,
        /// edit state for incremental parsing
        edit_state: usize 
    },
}