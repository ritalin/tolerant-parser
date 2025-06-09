use std::collections::HashMap;

use engine_core::{parser_engine::ParsingRuleSet, Engine};
use scanner_core::{iter::StatementScanner, Scanner};
use crate::{event_dispatcher::ParseEventDispatcher, metadata::{MetadataTable, StatementMetadataEntry}, node_handler::SyntaxTreeBuilder, parser::{parse_with_config_internal, DefaultParserStrategy, ParseError}, syntax_tree::{RowanLangageImpl, SyntaxNode, SyntaxNodeData, SyntaxTree}, ParserConfig};
use rayon::prelude::*;

pub struct Parser {
    engine: Engine,
}

impl Parser {
    pub fn new(engine: Engine) -> Self {
        Self { engine }
    }

    pub fn parse_with_config(&self, source: &str, config: ParserConfig) -> Result<Statements, ParseError> {
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

        Ok(Statements { members: statements, engine: self.engine.parsing_rules })
    }
}

struct Request {
    seq: usize,
    config: ParserConfig,
    scanner: StatementScanner,
    engine: ParsingRuleSet,
}

impl Request {
    fn parse(&mut self) -> Result<Statement, ParseError> {
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

        Ok(Statement { 
            seq: self.seq, 
            node, 
            adjusted_byte_offset: self.scanner.index(),
            metadata_entry: StatementMetadataEntry{ 
                map: metadata, 
                ..Default::default()
            },
        })
    }
}

pub struct Statement {
    seq: usize,
    node: rowan::NodeOrToken<rowan::GreenNode, rowan::GreenToken>,
    metadata_entry: StatementMetadataEntry,
    adjusted_byte_offset: usize,
}

impl Statement {
    pub fn into_root(self, engine: ParsingRuleSet) -> SyntaxNode {
        let red_node = rowan::api::SyntaxNode::<RowanLangageImpl>::new_root(self.node.clone().into_node().unwrap());

        let metadata_table = MetadataTable::new(vec![self.metadata_entry], StatementMetadataEntry::default());

        SyntaxNode::from_raw(SyntaxNodeData::new(red_node, std::rc::Rc::new(metadata_table), crate::ParseMode::ByStatement, engine))
    }

    #[inline]
    pub fn seq(&self) -> usize {
        self.seq
    }

    #[inline]
    pub fn byte_offset(&self) -> usize {
        self.adjusted_byte_offset
    }
}

impl std::fmt::Display for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let red_node = rowan::api::SyntaxNode::<RowanLangageImpl>::new_root(self.node.clone().into_node().unwrap());

        let mut next_token = red_node.first_token();
        while let Some(token) = next_token {
            write!(f, "{}", token.text())?;
            next_token = token.next_token();
        }

        Ok(())
    }
}

pub struct Statements {
   pub  members: Vec<Statement>,
   pub engine: ParsingRuleSet,
}

impl From<Statements> for SyntaxTree {
    fn from(value: Statements) -> Self {
        // FIXME: need to determine global char_offset
        todo!()
    }
}