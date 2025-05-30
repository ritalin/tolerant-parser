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
        let config = ParserConfig {
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        self.parse_with_config(source, config)
    }

    pub fn parse_with_config(&self, source: &str, config: ParserConfig) -> Result<super::syntax_tree::SyntaxTree, ParseError> {
        let terminate_symbol = self.engine.parsing_rules.statement_emit_config().to_symbol;

        let mut scanner = Scanner::create(source, 0, self.engine.scanning_rules.clone())?;
        let mut dispatcher = ParseEventDispatcher::new(0, config.mode.clone(), self.engine.parsing_rules);
        let mut tree_builder = SyntaxTreeBuilder::new(self.engine.parsing_rules, None);
        let mut recovery_handler = RecoveryEventDispatcher::new(config.penalty, self.engine.parsing_rules);

        loop { 
            let (event, lookahead) = match scanner.lookahead().cloned() {
                Some(lookahead) => (dispatcher.next(Some(lookahead.main.kind)), Some(lookahead)),
                None if dispatcher.has_next() => (dispatcher.next(None), None),
                None if config.mode == ParseMode::Full => (dispatcher.next(None), None),
                None => break,
            };

            match event {
                Ok(ParseEvent::Shift { .. }) => {
                    scanner.shift();
                    tree_builder.add_token_set(event?, lookahead.as_ref())?;
                }
                Ok(ParseEvent::Reduce { .. } | ParseEvent::PatchReduce { .. }) => {
                    tree_builder.add_node(event?)?;
                }
                Ok(ParseEvent::Emit { .. } | ParseEvent::InvalidEmit { .. }) => {
                    tree_builder.emit_statement(event?)?;

                    if config.mode == ParseMode::ByStatement {
                        dispatcher.flush_state();
                    }
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
                            dispatcher.post_recovery_event(&recovery_handler.handle_as_invalid(lookaheads));
                        }
                    }
                }
                Ok(ParseEvent::Invalid { .. }) if config.mode == ParseMode::ByStatement => {
                    scanner.shift();
                    tree_builder.add_token_set(event?, lookahead.as_ref())?;
                }
                Ok(ParseEvent::PatchDrop { .. } | ParseEvent::Invalid { .. }) => {
                    scanner.shift();
                    tree_builder.add_invisible_token_set(event?, lookahead.as_ref())?;
                }
                Ok(ParseEvent::PatchShift { .. }) => {
                    tree_builder.add_patch_shift_token_set(event?)?;
                }
                Err(err) => {
                    Err(err)?;
                }
            }
        }

        Err(ParseError::ByEvent(ParseEventError::NotAccept))
    }
}

/// Specifies the parsing mode behavior.
///
/// - `Full`: Parses the entire input as a single unit without explicitly
///   emitting individual statements. This typically results in a recursive
///   AST structure like:
///
///   ```text
///   root
///     └─ stmt_list
///           ├─ stmt
///           └─ stmt_list
///                 ├─ stmt
///                 └─ ...
///   ```
///
/// - `ByStatement`: Emits each statement as it is parsed, resetting internal
///   state between statements. This results in a flatter AST structure:
///
///   ```text
///   root
///     ├─ stmt
///     ├─ stmt
///     └─ ...
///   ```
#[derive(PartialEq, Clone)]
pub enum ParseMode {
    Full,
    ByStatement,
}

pub struct ParserConfig {
    pub mode: ParseMode,
    pub penalty: RecoveryPenalty,
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