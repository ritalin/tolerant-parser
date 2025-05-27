use engine_core::Engine;
use scanner_core::{Scanner, ScannerError};

use crate::{error_recovery::{RecoveryEventDispatcher, RecoveryPenalty}, event_dispatcher::{ParseEvent, ParseEventDispatcher, ParseEventError}, node_handler::{NodeBuildError, SyntaxTreeBuilder}};
pub(crate) use crate::error_recovery::RecoveryEvent;

pub struct DefaultPasrser {
    engine: Engine,
}

impl DefaultPasrser {
    pub fn new(engine: Engine) -> Self {
        Self { engine }
    }

    pub fn parse(&self, source: &str) -> Result<super::syntax_tree::SyntaxTree, ParseError> {
        let recovery_penalty = RecoveryPenalty {
            delete_slot: 3,
            shift_limit: 10,
            shift_decay: 0,
            next_shift_decay: 1,
            max_shift_packet_size: 10,
        };

        let terminate_symbol = self.engine.parsing_rules
            .statement_emit_config()
            .unwrap_or_else(|| self.engine.parsing_rules.full_emit_config())
            .to_symbol
        ;

        let mut scanner = Scanner::create(source, 0, self.engine.scanning_rules.clone())?;
        let mut dispatcher = ParseEventDispatcher::new(0, self.engine.parsing_rules);
        let mut tree_builder = SyntaxTreeBuilder::new(self.engine.parsing_rules, None);
        let mut recovery_handler = RecoveryEventDispatcher::new(recovery_penalty, self.engine.parsing_rules);

        loop { 
            let (event, lookahead) = match scanner.lookahead().cloned() {
                Some(lookahead) => (dispatcher.next(Some(lookahead.main.kind)), Some(lookahead)),
                None if dispatcher.has_next() => (dispatcher.next(None), None),
                None => break,
            };

            match event {
                Ok(ParseEvent::Shift { .. }) => {
                    scanner.shift();
                    tree_builder.add_token_set(event?, lookahead.as_ref())?;
                }
                Ok(ParseEvent::Reduce { .. }) => {
                    tree_builder.add_node(event?)?;
                }
                Ok(ParseEvent::Emit { .. }) => {
                    tree_builder.emit_statement(event?)?;
                    dispatcher.flush_state();
                }
                Ok(ParseEvent::Accept { .. }) => {
                    return Ok(tree_builder.build(event?)?);
                }
                Err(ParseEventError::RequestRecovery) => {
                    let state_stack = dispatcher.borrow_stack();
                    let lookaheads = scanner.prefetch(terminate_symbol);
                    match recovery_handler.handle(state_stack, lookaheads.clone()) {
                        Some(events) => {
                            // Recovery succeed
                            dispatcher.post_recovery_event(&events);
                        }
                        None => {
                            // Recovery failed
                            // dispatcher.push_recover_event(recovery_handler.handle_invalid(lookaheads));
                            todo!()
                        }
                    }
                }
                Ok(ParseEvent::RecoverDrop { .. }) => {
                    todo!()
                }
                Ok(ParseEvent::RecoverShift { .. }) => {
                    todo!()
                }
                Ok(ParseEvent::RecoverReduce { .. }) => {
                    todo!()
                }
                Ok(ParseEvent::Invalid { .. }) => {
                    todo!()
                }
                Err(_) => {
                    event?;
                }
            }
        }

        Err(ParseError::ByEvent(ParseEventError::NotAccept))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("inherited from ScannerError: {0}")]
    ByScanner(#[from] ScannerError),
    #[error("inherited from ParseEventError: {0}")]
    ByEvent(#[from] ParseEventError),
    #[error("inherited from NodeBuildError: {0}")]
    ByNode(#[from] NodeBuildError)
}