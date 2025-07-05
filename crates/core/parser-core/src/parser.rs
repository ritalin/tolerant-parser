use engine_core::{parser_engine::ParsingRuleSet, scanner_engine::CaseSensitivity, Engine, SyntaxKind};
use scanner_core::{Scanner, ScannerAccess, ScannerError};

use crate::{error_recovery::{RecoveryEventDispatcher, RecoveryPenalty}, event_dispatcher::{ParseEvent, ParseEventDispatcher, ParseEventError}, incremental::EditScope, node_handler::{NodeBuildError, SyntaxTreeBuilder}, syntax_tree::SyntaxTree};
pub(crate) use crate::error_recovery::RecoveryEvent;

pub struct DefaultPasrser {
    config: ParserConfig,
    engine: Engine,
}

impl DefaultPasrser {
    pub fn new(engine: Engine, config: ParserConfig) -> Self {
        Self { engine, config }
    }

    pub fn parse(&self, source: &str) -> Result<super::syntax_tree::SyntaxTree, ParseError> {
        let mut scanner = Scanner::create(source, 0, self.engine.scanning_rules.clone(), self.config.case_sensitive.clone())?;
        let mut dispatcher = ParseEventDispatcher::new(0, self.config.mode.clone(), self.engine.parsing_rules);
        let mut tree_builder = SyntaxTreeBuilder::new(self.engine.parsing_rules, self.config.mode.clone(), None);
 
        match parse_with_config_internal(&mut scanner, &mut dispatcher, &mut tree_builder, &self.config, self.engine.parsing_rules, DefaultParserStrategy)? {
            Some(ParseEvent::Accept { kind, last_state, edit_state  }) => {
                Ok(tree_builder.build(ParseEvent::Accept { kind, last_state, edit_state  })?)
            }
            Some(_) => {
                Err(NodeBuildError::NodeFailed)?
            }
            None => {
                Err(ParseEventError::NotAccept)?
            }
        }
    }

    pub fn incremental(&self, old_tree: &SyntaxTree, scope: EditScope) -> crate::incremental::Parser {
        crate::incremental::Parser::new(old_tree, scope, self.engine.clone())
    }
}

pub(crate) trait ParseStrategy {
    fn is_terminated_kind(&self, kind: SyntaxKind, scanner: &impl ScannerAccess) -> bool;
}

pub struct DefaultParserStrategy;

impl ParseStrategy for DefaultParserStrategy {
    fn is_terminated_kind(&self, _kind: SyntaxKind, _scanner: &impl ScannerAccess) -> bool {
        false
    }
}

pub(crate) fn parse_with_config_internal<S>(scanner: &mut S, dispatcher: &mut ParseEventDispatcher, tree_builder: &mut SyntaxTreeBuilder, config: &ParserConfig, engine: ParsingRuleSet, strategy: impl ParseStrategy) -> Result<Option<ParseEvent>, ParseError> 
where S: scanner_core::ScannerAccess
{
    let emit_symbol = engine.statement_emit_config();

    let mut recovery_handler = match config.mode {
        ParseMode::ByStatement => {
            RecoveryEventDispatcher::new(config.penalty.clone(), &[emit_symbol.from_symbol], engine)
        }
        ParseMode::Full => {
            RecoveryEventDispatcher::new(config.penalty.clone(), &[], engine)
        }
    };

    loop { 
        let (event, lookahead) = match scanner.lookahead().cloned() {
            Some(lookahead) => (dispatcher.next(Some(lookahead.main.kind)), Some(lookahead)),
            None if dispatcher.has_next() => (dispatcher.next(None), None),
            None if config.mode == ParseMode::Full => (dispatcher.next(None), None),
            None => break,
        };

        match event {
            Ok(ParseEvent::Shift { kind, .. }) => {
                scanner.shift();
                if strategy.is_terminated_kind(kind, scanner) {
                    // Do not shift and break loop
                    break;
                }

                tree_builder.add_token_set(event?, lookahead.as_ref())?;
            }
            Ok(ParseEvent::Reduce { .. } | ParseEvent::PatchReduce { .. }) => {
                tree_builder.add_node(event?)?;
            }
            Ok(ParseEvent::Emit { .. } | ParseEvent::PatchEmit { .. } | ParseEvent::InvalidEmit { .. }) => {
                tree_builder.emit_statement(event?)?;

                if config.mode == ParseMode::ByStatement {
                    dispatcher.flush_state();
                }
            }
            Ok(ParseEvent::Accept { .. }) => {
                return Ok(Some(event?));
            }
            Err(ParseEventError::RequestRecovery) => {
                let state_stack = dispatcher.borrow_stack();
                let lookaheads = scanner.prefetch_iter(emit_symbol.to_symbol);

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
                let last_offset = lookahead.map(|token| token.lowest_offset()).unwrap_or_default();
                tree_builder.add_patch_shift_token_set(event?, last_offset)?;
            }
            Err(err) => {
                Err(err)?;
            }
        }
    }
    
    Ok(None)
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
#[derive(PartialEq, Clone, Debug)]
pub enum ParseMode {
    Full,
    ByStatement,
}

#[derive(Clone)]
pub struct ParserConfig {
    pub mode: ParseMode,
    pub penalty: RecoveryPenalty,
    pub case_sensitive: CaseSensitivity,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            mode: ParseMode::ByStatement, 
            penalty: Default::default(), 
            case_sensitive: CaseSensitivity::Sensitive 
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("inherited from ScannerError: {0}")]
    ByScanner(#[from] ScannerError),
    #[error("inherited from ParseEventError: {0}")]
    ByEvent(#[from] ParseEventError),
    #[error("inherited from NodeBuildError: {0}")]
    ByNode(#[from] NodeBuildError),
     #[error("paralell parse failed: source: `{0}`")]
    Paraell(String)
}