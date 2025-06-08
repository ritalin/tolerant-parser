use std::{collections::HashMap, rc::Rc};
use engine_core::{parser_engine::ParsingRuleSet};
use scanner_core::{Scanner, ScannerAccess, StatementScannerView};
use crate::{event_dispatcher::ParseEventDispatcher, incremental::support::{IncludeEnd, IncrementalParserStrategy}, metadata::{MetadataTable, StatementMetadataEntry}, node_handler::SyntaxTreeBuilder, parser::ParseError, syntax_tree::{RowanLangageImpl, SyntaxTree}, NodeMetadata, NodeMetadataKey, ParserConfig};

pub mod support;

pub struct Parser {
    scope: EditScope,
    statements: Vec<rowan::api::SyntaxNode<RowanLangageImpl>>,
    root: rowan::api::SyntaxNode<RowanLangageImpl>,
    replace_from: usize,
    engine: engine_core::Engine,
    metadata_table: Rc<MetadataTable>,
}

impl Parser {
    pub fn new(old_tree: &SyntaxTree, scope: EditScope, engine: engine_core::Engine) -> Self {
        let root = old_tree.root().into_raw();
        let statements = find_edit_statements(old_tree, &scope).collect::<Vec<_>>();
        let replace_from = statements.first()
            .map(|stmt| stmt.index())
            .unwrap_or_else(|| {
                old_tree.metadata_table().index_at_offset(scope.start_byte_offset)
            })
        ;

        Self {
            scope,
            statements,
            root,
            replace_from,
            engine,
            metadata_table: old_tree.metadata_table()
        }
    }

    pub fn parse_with_config(&self, source: &str, config: ParserConfig) -> Result<SyntaxTree, crate::parser::ParseError> {
        // Determine first statement byte offset
        let mut global_byte_offset = self.metadata_table.statement_metadata(Some(self.replace_from)).byte_offset;
        
        let scan_from = self.statements.first().map(|stmt| usize::from(stmt.text_range().start())).unwrap_or(self.scope.start_byte_offset);
        let scanner = Scanner::create_without_scan(source, scan_from, self.engine.scanning_rules.clone())?;
        let emit_symbol = self.engine.parsing_rules.statement_emit_config().to_symbol;

        let stmts = self.statements.iter().map(Some).chain(std::iter::repeat(None));
        let scanners = scanner.statement_scanners(emit_symbol);
        
        let mut new_children = vec![];
        let mut new_metadata_table = vec![];

        for (stmt_scanner, stmt) in scanners.zip(stmts).filter(|(s, _)| s.as_view(std::ops::RangeFull).lookahead().is_some()) {
            let (new_stmt, new_metadata_entry) = match stmt {
                Some(stmt) => {
                    let stmt_index = stmt.index();
                    let old_stmt_range: std::ops::Range<usize> = stmt.text_range().into();
                    let (lowest, highest) = self.scope.adjust_range(
                        if self.scope.new_range().include_end().contains(&old_stmt_range.end) { self.scope.new_byte_len } else { self.scope.old_byte_len }, 
                        &stmt
                    );

                    let gardener = support::TreeGardener{ node: stmt.clone_subtree() };
                    // Find common anscestor
                    let common_anscestor = support::TreeGardener{ 
                        node: gardener.common_anscestor(
                            gardener.pick_token(lowest.into()), 
                            gardener.pick_token(highest.into()),
                            emit_symbol
                        )
                        .expect("At least, must exist")
                    };

                    // text_range is local coordicate because of clone_subtree()
                    let mut range: std::ops::Range<usize> = common_anscestor.node.text_range().into();
                    (range.start, range.end) = (range.start + global_byte_offset, range.end + global_byte_offset);
                    // Adgust by the edit distance
                    let anscestor_range = std::ops::Range {
                        start: range.start,
                        end: match self.scope.old_range().contains(&range.start) { 
                            true => range.end + old_stmt_range.start - stmt_scanner.index(),
                            false => range.end + self.scope.new_byte_len - self.scope.old_byte_len
                        },
                    };
                    
                    let strategy = common_anscestor.pick_terminate_kind(self.engine.parsing_rules);

                    // Memo: Because a last token is reduce, it scans one more token.
                    let scanner_view = stmt_scanner.as_view(anscestor_range.start..(anscestor_range.end + 1));
                    let old_metadata_map = &self.metadata_table.statement_metadata(Some(stmt_index));
                    let metadata = old_metadata_map.map
                        .get(&&NodeMetadataKey::from_raw_node(&common_anscestor.node, self.engine.parsing_rules))
                        .expect("All node have metadata")
                    ;

                    let (new_node, new_matadata_map) = parse_internal(scanner_view, &config, metadata.edit_state, strategy, self.engine.parsing_rules)?;
                    
                    let new_stmt = gardener.replace_with_new_node(new_node.clone(), &common_anscestor.node);
                    let metadata_entry = support::merge_metadata_map(
                        Some((common_anscestor.node, &old_metadata_map.map)),
                        new_node.as_node().map(|x| (x, new_matadata_map)).unwrap(),
                        global_byte_offset, metadata.char_offset,
                        self.engine.parsing_rules
                    );

                    (new_stmt, metadata_entry)
                }
                None => {
                    let scanner_view = stmt_scanner.as_view(std::ops::RangeFull);
                    let strategy = IncrementalParserStrategy::default_strategy(self.engine.parsing_rules);
                    let (new_stmt, new_matadata_map) = parse_internal(scanner_view, &config, 0, strategy, self.engine.parsing_rules)?;

                    let metadata_entry = support::merge_metadata_map(
                        None,
                        new_stmt.as_node().map(|x| (x, new_matadata_map)).unwrap(),
                        global_byte_offset, // Because lookahead is contained global_byte_offset
                        0, // Because always global_char_offset = 0 in the statement
                        self.engine.parsing_rules
                    );

                    (new_stmt, metadata_entry)
                }
            };

            let key = make_key_from_green_stmt(new_stmt.as_node(), self.engine.parsing_rules).expect("Statement key is not found");
            global_byte_offset += key.len; 

            new_children.push(new_stmt);
            new_metadata_table.push(new_metadata_entry);
        }
        
        let root = self.root.green().splice_children(self.replace_from..(self.replace_from + self.statements.len()), new_children);
        let metadata_table = update_metadata_table(
            root.children(), 
            self.metadata_table.as_ref(), new_metadata_table,
            self.replace_from..(self.replace_from + self.statements.len()),
            self.engine.parsing_rules
        );

        Ok(SyntaxTree::new(root, metadata_table, config.mode, self.engine.parsing_rules))
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct EditScope {
    pub start_byte_offset: usize,
    pub old_byte_len: usize,
    pub new_byte_len: usize,
}

impl EditScope {
    pub fn adjust_offset(&self, offset: usize) -> EditScope {
        let offset = usize::max(self.start_byte_offset, offset);
        Self {
            start_byte_offset: offset,
            old_byte_len: self.old_byte_len + offset - self.start_byte_offset,
            new_byte_len: self.new_byte_len + offset - self.start_byte_offset,
        }
    }

    pub fn adjust_range(&self, len: usize, node: &rowan::SyntaxNode<RowanLangageImpl>) -> (u32, u32) {
        let range = node.text_range();
        let lowest_offset = 
            u32::max(self.start_byte_offset as u32, range.start().into())
        ;
        let highest_offset = 
            u32::min(
                (self.start_byte_offset + len - 1) as u32, 
                range.end().into()
            )
        ;
        (lowest_offset, highest_offset)
    }

    pub fn old_range(&self) -> std::ops::Range<usize> {
        std::ops::Range { start: self.start_byte_offset, end: self.start_byte_offset + self.old_byte_len }
    }

    pub fn new_range(&self) -> std::ops::Range<usize> {
        std::ops::Range { start: self.start_byte_offset, end: self.start_byte_offset + self.new_byte_len }
    }
}

pub fn find_edit_statements(old_tree: &SyntaxTree, scope: &EditScope) -> impl Iterator<Item = rowan::api::SyntaxNode<RowanLangageImpl>> {
    use crate::syntax_tree::MetadataAccess;

    let (range_from, range_to) = (scope.start_byte_offset, scope.start_byte_offset + scope.old_byte_len);

    old_tree.root().children()
    .filter_map(|node| match node {
        crate::syntax_tree::SyntaxElementDef::Node(node) => {
            Some((node.clone(), node.metadata_key()))
        }
        crate::syntax_tree::SyntaxElementDef::TokenSet(_) => None
    })
    .skip_while(move |(_, key)| {
        key.offset + key.len <= range_from
    })
    .take_while(move |(_, key)| {
        key.offset < range_to
    })
    .map(|(node, _)| node.into_raw())
}

fn parse_internal(
    mut scanner: StatementScannerView, 
    config: &ParserConfig,
    edit_state: usize,
    parse_strategy: impl crate::parser::ParseStrategy,
    engine: ParsingRuleSet) -> Result<(rowan::NodeOrToken<rowan::GreenNode, rowan::GreenToken>, HashMap<NodeMetadataKey, NodeMetadata>), crate::parser::ParseError> 
{
    let mut dispatcher = ParseEventDispatcher::new(edit_state, config.mode.clone(), engine);
    let mut tree_builder = SyntaxTreeBuilder::new(engine, config.mode.clone(), None);

    match super::parser::parse_with_config_internal(&mut scanner, &mut dispatcher, &mut tree_builder, config, engine, parse_strategy) {
        Ok(_) => {
            Ok(tree_builder.build_branch()?)
        }
        Err(ParseError::ByEvent(crate::event_dispatcher::ParseEventError::NoMoreState{..})) => {
            // In incremental parsing mode, this mismatch may occur when the target node starts
            // from the middle of a production rule (i.e., not from the first symbol).
            // In such cases, a reduce may expect more items to pop than are available.
            //
            // This is allowed in incremental mode, since the parser stack doesn't contain
            // the nodes before the target.
            Ok(tree_builder.build_branch()?)
        }
        Err(err) => Err(err)
    }
}

fn update_metadata_table<'a>(
    children: impl Iterator<Item = rowan::NodeOrToken<&'a rowan::GreenNodeData , &'a rowan::GreenTokenData>>,
    old_table: &MetadataTable, 
    changed_maps: Vec<StatementMetadataEntry>, 
    replace_range: std::ops::Range<usize>,
    engine: ParsingRuleSet) -> MetadataTable
{
    let mut metadata_members = old_table.clone_members();
    let (mut byte_offset, mut char_offset) = (0, 0);

    metadata_members.splice(replace_range, changed_maps);

    for (i, child) in children.enumerate() {
        if let Some(entry) = metadata_members.get_mut(i) {
            entry.byte_offset = byte_offset;
            entry.char_offset = char_offset;
        }

        let key = NodeMetadataKey{ 
            kind: engine.from_kind_id(child.kind().0 as u32), 
            offset: 0, 
            len: child.text_len().into(), 
            is_leaf: false 
        };
        let metadata = metadata_members[i].map.get(&key)
            .expect(&format!("Failed to update metadata of incremental parse. (key: {:?})", key))
        ;
        byte_offset += key.len;
        char_offset += metadata.char_len;
    }
    
    // Update root node metadata
    let (key, metadata) = old_table.statement_metadata(None).map.iter()
        .map(|(key, metadata)| {
            (NodeMetadataKey{ len: byte_offset, ..key.clone() }, NodeMetadata{ char_len: char_offset, ..metadata.clone() })
        })
        .next()
        .expect("Not found Root node metadata")
    ;

    let root_entry = StatementMetadataEntry{ map: HashMap::from([(key, metadata)]), ..Default::default() };

    MetadataTable::new(metadata_members, root_entry)
}

fn make_key_from_green_stmt(stmt: Option<&rowan::GreenNode>, engine: ParsingRuleSet) -> Option<NodeMetadataKey> {
    stmt.map(|node| {
        NodeMetadataKey{ 
            kind: engine.from_kind_id(node.kind().0 as u32), 
            offset: 0, 
            len: node.text_len().into(), 
            is_leaf: false 
        }
    })
}