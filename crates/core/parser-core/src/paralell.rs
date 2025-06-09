use std::collections::HashMap;

use engine_core::{parser_engine::ParsingRuleSet, Engine};
use scanner_core::{iter::StatementScanner, Scanner};
use crate::{event_dispatcher::ParseEventDispatcher, metadata::{MetadataTable, StatementMetadataEntry}, node_handler::SyntaxTreeBuilder, parser::{parse_with_config_internal, DefaultParserStrategy, ParseError}, syntax_tree::{RowanLangageImpl, SyntaxNode, SyntaxNodeData, SyntaxTree}, NodeMetadata, NodeMetadataKey, ParseMode, ParserConfig};
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

        Ok(Statements { members: statements, engine: self.engine.parsing_rules, parse_mode: config.mode.clone() })
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
    pub fn into_root(self, parse_mode: ParseMode, engine: ParsingRuleSet) -> SyntaxNode {
        let red_node = rowan::api::SyntaxNode::<RowanLangageImpl>::new_root(self.node.clone().into_node().unwrap());

        let metadata_table = MetadataTable::new(vec![self.metadata_entry], StatementMetadataEntry::default());

        SyntaxNode::from_raw(SyntaxNodeData::new(red_node, std::rc::Rc::new(metadata_table), parse_mode, engine))
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
   pub parse_mode: ParseMode,
}

impl From<Statements> for SyntaxTree {
    fn from(value: Statements) -> Self {
        let mut members = value.members;
        members.sort_by(|lhs, rhs| lhs.seq().cmp(&rhs.seq()));

        let mut global_byte_offset = 0;
        let mut global_char_offset = 0;

        let (children, metadata_entriess): (Vec<rowan::NodeOrToken<rowan::GreenNode, rowan::GreenToken>>, Vec<StatementMetadataEntry>) = 
            members.into_iter()
            .map(|member| {
                let metadata_entry = StatementMetadataEntry{ 
                    byte_offset: global_byte_offset, 
                    char_offset: global_char_offset, 
                    map: member.metadata_entry.map 
                };
                global_byte_offset += usize::from(member.node.text_len());
                global_char_offset += measure_statement_char_len(std::borrow::Borrow::borrow(member.node.as_node().unwrap()));

                (member.node, metadata_entry)
            })
            .unzip()
        ;

        let root_key = NodeMetadataKey{ 
            kind: value.engine.full_emit_config().from_symbol, 
            offset: 0, len: global_byte_offset, is_leaf: false 
        };
        let root_metadata = NodeMetadata{ 
            edit_state: 0, 
            node_type: crate::NodeType::Node, 
            patch: crate::PatchAction::None, 
            char_offset: 0, 
            char_len: global_char_offset 
        };
        let root_metadata_entry = StatementMetadataEntry{ 
            map: HashMap::from([(root_key, root_metadata)]), 
            ..Default::default() 
        };
        let root_kind_id = rowan::SyntaxKind(value.engine.full_emit_config().from_symbol.id as u16);
        let root_node = rowan::GreenNode::new(root_kind_id, children);

        let metadata_table = MetadataTable::new(metadata_entriess, root_metadata_entry);

        SyntaxTree::new(root_node, metadata_table, value.parse_mode, value.engine)
    }
}

fn measure_statement_char_len(root: &rowan::GreenNodeData) -> usize {
    let mut stack = vec![rowan::NodeOrToken::Node(root)];
    let mut size = 0;

    while let Some(node) = stack.pop() {
        match node {
            rowan::NodeOrToken::Node(node) => {
                stack.extend(node.children());
            }
            rowan::NodeOrToken::Token(token) => {
                size += token.text().chars().count();
            }
        }
    }

    size
}