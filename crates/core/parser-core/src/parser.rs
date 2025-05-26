use engine_core::Engine;
use scanner_core::{Scanner, ScannerError};

use crate::{event_dispatcher::{ParseEvent, ParseEventDispatcher, ParseEventError}, node_handler::{NodeBuildError, SyntaxTreeBuilder}};

pub struct DefaultPasrser {
    engine: Engine,
}

impl DefaultPasrser {
    pub fn new(engine: Engine) -> Self {
        Self { engine }
    }

    pub fn parse(&self, source: &str) -> Result<super::syntax_tree::SyntaxTree, ParseError> {
        let mut scanner = Scanner::create(source, 0, self.engine.scanning_rules.clone())?;
        let mut dispatcher = ParseEventDispatcher::new(0, self.engine.parsing_rules);
        let mut tree_builder = SyntaxTreeBuilder::new(self.engine.parsing_rules, None);

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
                    // match 
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