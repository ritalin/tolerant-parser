use std::{collections::HashMap, rc::Rc};
use engine_core::{parser_engine::ParsingRuleSet};
use scanner_core::{Scanner, ScannerAccess, StatementScannerView};
use crate::{event_dispatcher::ParseEventDispatcher, incremental::support::{IncludeEnd, IncrementalParserStrategy}, metadata::MetadataTable, node_handler::SyntaxTreeBuilder, parser::ParseError, syntax_tree::{FragmentNodeMetadataKey, MetadataAccess, SyntaxElement, SyntaxFragment, SyntaxFragmentBatch, SyntaxNode, SyntaxTree}, NodeMetadata, NodeMetadataKey, ParserConfig};

pub mod support;

pub struct Parser {
    scope: EditScope,
    statements: Vec<SyntaxNode>,
    following_statement: Option<SyntaxElement>,
    replace_from: usize,
    engine: engine_core::Engine,
    metadata_table: Rc<MetadataTable>,
}

impl Parser {
    pub fn new(old_tree: &SyntaxTree, scope: EditScope, engine: engine_core::Engine) -> Self {
        let statements = find_edit_statements(old_tree, &scope).collect::<Vec<_>>();
        let replace_from = statements.first()
            .map(|stmt| stmt.into_raw().index())
            .unwrap_or_else(|| {
                old_tree.metadata_table().index_at_offset(scope.start_char_offset)
            })
        ;
        let eof_statement = old_tree.root().children().skip(replace_from + statements.len()).next();

        Self {
            scope,
            statements,
            following_statement: eof_statement,
            replace_from,
            engine,
            metadata_table: old_tree.metadata_table(),
        }
    }

    pub fn parse_with_config(&self, source: &str, config: ParserConfig) -> Result<Vec<SyntaxFragmentBatch>, crate::parser::ParseError> {
        // Determine first statement byte offset
        let mut global_byte_offset = self.metadata_table.statement_metadata(Some(self.replace_from)).global_offset.of_byte;
        
        // Determin scanning byte offset
        let scan_from = self.metadata_table.statement_metadata(Some(self.replace_from)).global_offset.of_byte;
        let scan_to = convert_offset_from_utf16_to_byte(self.scope.new_char_range().end, &source).unwrap_or(source.len());

        let scanner = Scanner::create_without_scan(source, scan_from, self.engine.scanning_rules.clone())?;
        let emit_region = self.engine.parsing_rules.statement_emit_config();
        // enumerate scanners except for over the new byte offset scope.
        // This includes `EOF` only statement.
        let scanners = scanner.statement_scanners(emit_region.to_symbol)
            .take_while(|scanner| scanner.index() < scan_to)
        ;

        let old_char_range = self.scope.old_char_range();
        // Determine old edit range start
        let old_token_start = 
            token_offset_at_scope(self.statements.first(), old_char_range.start, self.following_statement.as_ref())
            .unwrap_or_default()
        ;
        // Determine old edit range end
        let old_token_end = 
            token_offset_at_scope(self.statements.last(), old_char_range.end, self.following_statement.as_ref())
            .unwrap_or(source.len())
        ;

        let new_scope_range = old_token_start..scan_to;
        let old_scope_range = old_token_start..old_token_end;

        let stmts = self.statements.iter().map(Some).chain(std::iter::repeat(None));
        
        let mut fragments = vec![];
        let mut old_first_fragment_key = None;

        for (stmt_scanner, stmt) in scanners.zip(stmts).filter(|(s, _)| s.as_view(std::ops::RangeFull).lookahead().is_some()) {
            let (new_stmt, new_metadata_entry, new_node_key) = match stmt {
                Some(stmt) => {
                    let stmt = stmt.into_raw();
                    let stmt_index = stmt.index();
                    let old_stmt_range: std::ops::Range<usize> = stmt.text_range().into();
                    
                    let stmt_edit_range = support::adjust_edit_range(
                        if new_scope_range.clone().include_end().contains(&old_stmt_range.end) { &new_scope_range } else { &old_scope_range },
                        &stmt
                    );

                    let gardener = support::TreeGardener{ node: stmt.clone_subtree() };
                    // Find common anscestor
                    let common_anscestor = support::TreeGardener{ 
                        node: gardener.common_anscestor(
                            gardener.pick_token(rowan::TextSize::new(stmt_edit_range.start as u32)), 
                            gardener.pick_token(rowan::TextSize::new(stmt_edit_range.end as u32)),
                            emit_region.to_symbol
                        )
                        .expect("A metadata definitely must exist")
                    };

                    // text_range is local coordicate because of clone_subtree()
                    let mut range: std::ops::Range<usize> = common_anscestor.node.text_range().into();
                    (range.start, range.end) = (range.start + global_byte_offset, range.end + global_byte_offset);
                    // Adgust by the edit distance
                    let anscestor_range = std::ops::Range {
                        start: range.start,
                        end: match old_scope_range.contains(&range.start) { 
                            true => range.end + old_stmt_range.start - stmt_scanner.index(),
                            false => range.end + new_scope_range.end - old_scope_range.end
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
                    let new_key = common_anscestor.new_node_key(&new_node, self.engine.parsing_rules);
                    
                    if old_first_fragment_key.is_none() {
                        // If it's assigned, assigns the statement key as the first fragment key
                        old_first_fragment_key = Some(FragmentNodeMetadataKey{
                            key: NodeMetadataKey::from_raw_node(&common_anscestor.node, self.engine.parsing_rules),
                            is_eof: false,
                        });
                    }

                    let new_stmt = gardener.replace_with_new_node(new_node.clone(), &common_anscestor.node);
                    let metadata_entry = support::merge_metadata_map(
                        Some((common_anscestor.node, &old_metadata_map.map)),
                        (&new_node, new_matadata_map),
                        global_byte_offset, metadata.char_offset,
                        self.engine.parsing_rules
                    );

                    (new_stmt, metadata_entry, new_key)
                }
                None => {
                    let scanner_view = stmt_scanner.as_view(std::ops::RangeFull);
                    let strategy = IncrementalParserStrategy::default_strategy(self.engine.parsing_rules);

                    let (new_stmt, new_matadata_map) = parse_internal(scanner_view, &config, 0, strategy, self.engine.parsing_rules)?;
                    let new_key = NodeMetadataKey::from_green_node(&new_stmt, 0, self.engine.parsing_rules);

                    if old_first_fragment_key.is_none() {
                        // If it's assgned, assigns the following statement ( = EOF only statement) as the first fragment key
                        old_first_fragment_key = self.following_statement.as_ref().map(|el| {
                            FragmentNodeMetadataKey{ key: el.metadata_key(), is_eof: true }
                        });
                    }

                    let metadata_entry = support::merge_metadata_map(
                        None,
                        (&new_stmt, new_matadata_map),
                        global_byte_offset, // Because lookahead is contained global_byte_offset
                        0, // Because always global_char_offset = 0 in the statement
                        self.engine.parsing_rules
                    );

                    (new_stmt.clone(), metadata_entry, new_key)
                }
            };

            global_byte_offset += usize::from(new_stmt.text_len()); 

            fragments.push(SyntaxFragment::new(fragments.len(), new_stmt, new_metadata_entry, new_node_key))
        }

        let batch = SyntaxFragmentBatch{
            fragments,
            replace_from: self.replace_from,
            replace_size: self.statements.len(),
            old_first_fragment_key,
            engine: self.engine.parsing_rules,
        };

        Ok(vec![batch])
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct EditScope {
    /// editing start offset (UTF-16 char unit base)
    pub start_char_offset: usize,
    /// old editing length (UTF-16 char units)
    pub old_char_len: usize,
    /// new editing length (UTF-16 char units)
    pub new_char_len: usize,
}

impl EditScope {
    pub fn adjust_offset(&self, offset: usize) -> EditScope {
        let offset = usize::max(self.start_char_offset, offset);
        Self {
            start_char_offset: offset,
            old_char_len: self.old_char_len + offset - self.start_char_offset,
            new_char_len: self.new_char_len + offset - self.start_char_offset,
        }
    }

    pub fn old_char_range(&self) -> std::ops::Range<usize> {
        std::ops::Range { start: self.start_char_offset, end: self.start_char_offset + self.old_char_len }
    }

    pub fn new_char_range(&self) -> std::ops::Range<usize> {
        std::ops::Range { start: self.start_char_offset, end: self.start_char_offset + self.new_char_len }
    }
}

pub fn find_edit_statements(old_tree: &SyntaxTree, scope: &EditScope) -> impl Iterator<Item = SyntaxNode> {
    use crate::syntax_tree::MetadataAccess;

    let (range_from, range_to) = (scope.start_char_offset, scope.start_char_offset + scope.old_char_len);

    old_tree.root().children()
    .filter_map(|node| match node {
        crate::syntax_tree::SyntaxElementDef::Node(node) => {
            Some((node.clone(), node.metadata()))
        }
        crate::syntax_tree::SyntaxElementDef::TokenSet(_) => None
    })
    .skip_while(move |(_, metadata)| {
        metadata.char_offset + metadata.char_len <= range_from
    })
    .take_while(move |(_, metadata)| {
        metadata.char_offset < range_to
    })
    .map(|(node, _)| node)
}

fn convert_offset_from_utf16_to_byte(char_offset: usize, source: &str) -> Option<usize> {
    let mut current_char_offset = 0;

    for (byte_offset, ch) in source.char_indices() {
        if current_char_offset == char_offset { return Some(byte_offset); }

        current_char_offset += ch.len_utf16();
        // if over current_char_offset, middle of surrogate pair
        if current_char_offset > char_offset { return Some(byte_offset); }
    }

    None
}

fn token_offset_at_scope(stmt: Option<&SyntaxNode>, char_offset: usize, following_stmt: Option<&SyntaxElement>) -> Option<usize> {
    stmt.and_then(|stmt| stmt.token_at_utf16_offset(char_offset))
        .map(|token| token.metadata_key())
        .or_else(|| following_stmt.map(|stmt| stmt.metadata_key()))
        .map(|key| key.offset)
}

fn parse_internal(
    mut scanner: StatementScannerView, 
    config: &ParserConfig,
    edit_state: usize,
    parse_strategy: impl crate::parser::ParseStrategy,
    engine: ParsingRuleSet) -> Result<(rowan::GreenNode, HashMap<NodeMetadataKey, NodeMetadata>), crate::parser::ParseError> 
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
