use std::collections::HashMap;

use engine_core::{parser_engine::ParsingRuleSet, Engine};
use scanner_core::{iter::StatementScanner, Scanner};
use crate::{event_dispatcher::ParseEventDispatcher, metadata::StatementMetadataEntry, node_handler::SyntaxTreeBuilder, parser::{parse_with_config_internal, DefaultParserStrategy, ParseError}, syntax_tree::{SyntaxFragment, SyntaxFragmentBatch}, ParserConfig};
use rayon::prelude::*;

pub struct Parser {
    engine: Engine,
}

impl Parser {
    pub fn new(engine: Engine) -> Self {
        Self { engine }
    }

    pub fn parse_with_config(&self, source: &str, config: ParserConfig) -> Result<SyntaxFragmentBatch, ParseError> {
        let scanner = Scanner::create_without_scan(source, 0, self.engine.scanning_rules.clone())?;
        
        let scanners = scanner.statement_scanners(self.engine.parsing_rules.statement_emit_config().to_symbol)
            .enumerate()   
            .collect::<Vec<_>>()
        ;

        let statements = scanners.into_iter().par_bridge()
        .map(|(seq, scanner)| {
            let mut req = Request{ seq, config: config.clone(), scanner, engine: self.engine.parsing_rules };
            req.parse()
        })
        .collect::<Result<Vec<_>, _>>()?;

        Ok(SyntaxFragmentBatch { members: statements, engine: self.engine.parsing_rules, parse_mode: config.mode.clone() })
    }
}

struct Request {
    seq: usize,
    config: ParserConfig,
    scanner: StatementScanner,
    engine: ParsingRuleSet,
}

impl Request {
    fn parse(&mut self) -> Result<SyntaxFragment, ParseError> {
        let mut dispatcher = ParseEventDispatcher::new(0, self.config.mode.clone(), self.engine);
        let mut tree_builder = SyntaxTreeBuilder::new(self.engine, self.config.mode.clone(), None);
        
        parse_with_config_internal(&mut self.scanner.as_view(..), &mut dispatcher, &mut tree_builder, &self.config, self.engine, DefaultParserStrategy)?;
        let (node, metadata) = tree_builder.build_branch()?;

        let global_byte_offset = self.scanner.index();
        let metadata = metadata.into_iter()
            .map(|(key, metadata)| {
                (key.into_local(global_byte_offset), metadata)
            })
            .collect::<HashMap<_, _>>()
        ;

        Ok(SyntaxFragment::new(self.seq, node, metadata).adjust_byte_offset(self.scanner.index()))
    }
}
