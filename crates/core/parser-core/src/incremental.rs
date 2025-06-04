use std::{collections::HashMap, rc::Rc};
use engine_core::{parser_engine::ParsingRuleSet, SyntaxKind};
use scanner_core::{Scanner, ScannerAccess, StatementScannerView};
use crate::{event_dispatcher::ParseEventDispatcher, metadata::StatementMetadataMap, node_handler::SyntaxTreeBuilder, syntax_tree::{RowanLangageImpl, SyntaxTree}, NodeId, NodeMetadata, NodeMetadataKey, ParserConfig};

pub mod support;

pub struct Parser {
    scope: EditScope,
    statements: Vec<rowan::api::SyntaxNode<RowanLangageImpl>>,
    root: rowan::api::SyntaxNode<RowanLangageImpl>,
    replace_from: usize,
    engine: engine_core::Engine,
    metadata_table: Rc<Vec<StatementMetadataMap>>,
}

impl Parser {
    pub fn new(old_tree: &SyntaxTree, scope: EditScope, engine: engine_core::Engine) -> Self {
        let root = old_tree.root().into_raw();
        let statements = find_edit_statements(old_tree, &scope).collect::<Vec<_>>();
        let replace_from = statements.first().map(|stmt| stmt.index()).unwrap_or_default();

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
        let scan_from = self.statements.first().map(|stmt| u32::from(stmt.text_range().start())).unwrap_or_default();
        let scanner = Scanner::create_without_scan(source, scan_from, self.engine.scanning_rules.clone())?;
        let emit_symbol = self.engine.parsing_rules.statement_emit_config().to_symbol;
        let full_emit_symbol = self.engine.parsing_rules.full_emit_config().to_symbol;

        // Determine first statement offset (byte/char)
        let (mut global_byte_offset, mut global_char_offset) = self.metadata_table.get(self.replace_from + 1)
            .map(|entry| (entry.byte_offset, entry.char_offset))
            .unwrap_or_else(|| (0, 0))
        ;
        
        let stmts = self.statements.iter().map(Some).chain(std::iter::repeat(None));
        let scanners = scanner.statement_scanners(emit_symbol);
        
        let mut new_children = vec![];
        let mut new_metadata_table = vec![];

        for (stmt_scanner, stmt) in scanners.zip(stmts).filter(|(s, _)| s.as_view(std::ops::RangeFull).lookahead().is_some()) {
            let (new_stmt, new_metadata_entry) = match stmt {
                Some(stmt) => {
                    let stmt_index = stmt.index();
                    let stmt = stmt.clone_subtree();
                    let gardener = support::TreeGardener{ node: stmt.clone() };
                    let (lowest, highest) = self.scope.adjust_offset(self.scope.old_byte_len, stmt);
                    // Find common anscestor
                    let common_anscestor = support::TreeGardener{ 
                        node: gardener.common_anscestor(
                            gardener.pick_token(lowest.into()), 
                            gardener.pick_token(highest.into()),
                            emit_symbol
                        )
                        .expect("At least, must exist")
                    };

                    let mut anscestor_range: std::ops::Range<usize> = common_anscestor.node.text_range().into();
                    // Adgust by the edit distance
                    anscestor_range.end = anscestor_range.end - self.scope.old_byte_len + self.scope.new_byte_len;
                    
                    let terminate_kind = common_anscestor.pick_terminate_kind(self.engine.parsing_rules);

                    // Memo: Because A last token is reduce, it scans one more token.
                    let stmt_scanner = stmt_scanner.as_view(anscestor_range.start..(anscestor_range.end + 1));
                    let old_metadata_map = &self.metadata_table[stmt_index + 1]; // Index: 1 is a root node metadata
                    let (_, metadata) = old_metadata_map.map
                        .get(&&NodeMetadataKey::from_raw_node(&common_anscestor.node, self.engine.parsing_rules))
                        .expect("All node have metadata")
                    ;

                    let (new_node, new_matadata_map) = parse_internal(stmt_scanner, &config, metadata.edit_state, terminate_kind, self.engine.parsing_rules)?;
                    
                    let new_stmt = gardener.replace_with_new_node(new_node.clone(), &common_anscestor.node);
                    let metadata_entry = support::merge_metadata_map(
                        Some((common_anscestor.node, &old_metadata_map.map)),
                        new_node.into_node().map(|x| (x, new_matadata_map)).unwrap(),
                        global_byte_offset, global_char_offset, metadata.char_offset,
                        self.engine.parsing_rules
                    );

                    (new_stmt, metadata_entry)
                }
                None => {
                    let stmt_scanner = stmt_scanner.as_view(std::ops::RangeFull);
                    let (new_stmt, new_matadata_map) = parse_internal(stmt_scanner, &config, 0, full_emit_symbol, self.engine.parsing_rules)?;

                    let metadata_entry = support::merge_metadata_map(
                        None,
                        new_stmt.clone().into_node().map(|x| (x, new_matadata_map)).unwrap(),
                        global_byte_offset, global_char_offset, 0,
                        self.engine.parsing_rules
                    );

                    (new_stmt, metadata_entry)
                }
            };

            let key = make_key_from_green_stmt(new_stmt.as_node(), self.engine.parsing_rules).expect("Statement key is not found");
            let (_, metadata) = new_metadata_entry.map.get(&key).expect("Statement metadata is not found");
            global_byte_offset += key.len; 
            global_char_offset = metadata.char_len;

            new_children.push(new_stmt);
            new_metadata_table.push(new_metadata_entry);
        }
        
        let root = self.root.green().splice_children(self.replace_from..(self.replace_from + self.statements.len()), new_children);
        let metadata_table = update_metadata_table(
            root.children(), 
            self.metadata_table.as_ref(), new_metadata_table,
            self.replace_from, self.statements.len(),
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
    pub fn adjust_offset(&self, len: usize, node: rowan::SyntaxNode<RowanLangageImpl>) -> (u32, u32) {
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
        key.offset + key.len < range_from
    })
    .take_while(move |(_, key)| {
        key.offset < range_to
    })
    .map(|(node, _)| node.into_raw())
}

struct IncrementalParserStrategy {
    terminate_kind: SyntaxKind,
}

impl crate::parser::ParseStrategy for IncrementalParserStrategy {
    fn is_terminated_kind(&self, kind: SyntaxKind) -> bool {
        self.terminate_kind == kind
    }
}

fn parse_internal(
    mut scanner: StatementScannerView, 
    config: &ParserConfig,
    edit_state: usize,
    terminate_kind: SyntaxKind,
    engine: ParsingRuleSet) -> Result<(rowan::NodeOrToken<rowan::GreenNode, rowan::GreenToken>, HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>), crate::parser::ParseError> 
{
    let mut dispatcher = ParseEventDispatcher::new(edit_state, config.mode.clone(), engine);
    let mut tree_builder = SyntaxTreeBuilder::new(engine, config.mode.clone(), None);
    let strategy = IncrementalParserStrategy{ terminate_kind };

    super::parser::parse_with_config_internal(&mut scanner, &mut dispatcher, &mut tree_builder, config, engine, strategy)?;

    Ok(tree_builder.build_branch()?)
}

fn update_metadata_table<'a>(
    children: impl Iterator<Item = rowan::NodeOrToken<&'a rowan::GreenNodeData , &'a rowan::GreenTokenData>>,
    old_table: &[StatementMetadataMap], 
    changed_maps: Vec<StatementMetadataMap>, 
    replace_from: usize, old_len: usize,
    engine: ParsingRuleSet) -> Vec<StatementMetadataMap>
{
    let mut metadata_table = Vec::from_iter(old_table.iter().cloned());
    let (mut byte_offset, mut char_offset) = (0, 0);

    metadata_table.splice((replace_from + 1)..(replace_from + 1 + old_len), changed_maps);

    for (i, child) in children.enumerate() {
        if let Some(entry) = metadata_table.get_mut(i + 1) {
            entry.byte_offset = byte_offset;
            entry.char_offset = char_offset;
        }

        let key = NodeMetadataKey{ 
            kind: engine.from_kind_id(child.kind().0 as u32), 
            offset: 0, 
            len: child.text_len().into(), 
            is_leaf: false 
        };
        let (_, metadata) = metadata_table[i + 1].map.get(&key).expect(&format!("Failed to update metadata of incremental parse. (key: {:?})", key));
        byte_offset += key.len;
        char_offset += metadata.char_len;
    }
    
    // Update root node metadata
    let root_metadata = &old_table[0];
    let (mut key, (id, mut metadata)) = root_metadata.map.iter()
        .map(|(key, (id, metadata))| (key.clone(), (id.clone(), metadata.clone())))
        .next()
        .expect("Not found Root node metadata")
    ;
    key.len = byte_offset;
    metadata.char_len = char_offset;
    metadata_table[0].map.insert(key, (id, metadata));

    metadata_table
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