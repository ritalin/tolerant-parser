use std::{collections::HashMap, rc::Rc};
use crate::core::engine_core::{self, parser_engine::ParsingRuleSet};
use crate::core::scanner_core::{Scanner, StatementScannerView};
use crate::core::parser_core::{self, event_dispatcher::ParseEventDispatcher, incremental::{edit_hint::SlotEvent, support::{IncludeEnd, IncrementalParserStrategy}}, metadata::MetadataTable, node_handler::SyntaxTreeBuilder, parser::ParseError, syntax_tree::{FragmentNodeMetadataKey, MetadataAccess, SyntaxElement, SyntaxFragment, SyntaxFragmentBatch, SyntaxTree}, NodeMetadata, NodeMetadataKey, ParserConfig, PatchAction};

pub mod support;
pub mod edit_hint;

pub struct Parser {
    scope: EditScope,
    edit_hint: edit_hint::EditHint,
    following_statement: Option<SyntaxElement>,
    engine: engine_core::Engine,
    metadata_table: Rc<MetadataTable>,
    config: ParserConfig,
}

impl Parser {
    pub fn new(old_tree: &SyntaxTree, scope: EditScope, engine: engine_core::Engine, config: ParserConfig) -> Self {
        let eof_statement = old_tree.root().children().last();
        let edit_hint = edit_hint::EditHint::new(old_tree, scope.old_char_range());

        Self {
            scope,
            edit_hint,
            following_statement: eof_statement,
            engine,
            metadata_table: old_tree.metadata_table(),
            config,
        }
    }

    pub fn parse(&self, source: &str) -> Result<Vec<SyntaxFragmentBatch>, parser_core::parser::ParseError> {
        let new_edit_byte_range = convert_from_utf16_to_byte_range(self.scope.new_char_range(), source);

        // Determin scanning byte offset
        let scan_from = self.edit_hint.scan_from();
        let scan_to = new_edit_byte_range.end;

        let scanner = Scanner::create_without_scan(source, scan_from, self.engine.scanning_rules.clone(), self.config.case_sensitive.clone())?;

        let emit_region = self.engine.parsing_rules.statement_emit_config();
        let full_emit_region = self.engine.parsing_rules.full_emit_config();

        // enumerate scanners except for over the new byte offset scope.
        let scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let slots = self.edit_hint.eval_hint(scanners, new_edit_byte_range.clone());

        // Determine first statement byte offset
        let mut global_byte_offset = self.metadata_table.statement_metadata(Some(slots.replace_from)).global_offset.of_byte;

        let old_scope_range = slots.replace_byte_range.unwrap_or_else(|| 0..source.len());
        let new_scope_range = old_scope_range.start..scan_to;
        
        let mut fragments = vec![];
        let mut old_first_fragment_key = None;

        for event in &slots.events {
            let (new_stmt, new_metadata_entry, new_node_key) = match event {
                SlotEvent::Replacing {node: stmt, scanner: stmt_scanner } => {
                    let old_stmt_range: std::ops::Range<usize> = stmt.metadata_key().byte_range();
                    let stmt_edit_range = support::adjust_edit_range(
                        if new_scope_range.clone().include_end().contains(&old_stmt_range.end) { &new_scope_range } else { &old_scope_range },
                        &old_stmt_range
                    );

                    let gardener = support::TreeGardener::as_subtree(&stmt);
                    // Find common anscestor
                    let common_anscestor = match stmt.metadata().patch {
                        PatchAction::None => {
                            // No error in  the previous parsing
                            gardener.common_anscestor(
                                gardener.pick_token(stmt_edit_range.start), 
                                gardener.pick_token(stmt_edit_range.end),
                                emit_region.to_symbol
                            )
                            .expect("A metadata definitely must exist")
                        }
                        _ => {
                            // Parse whole statement
                            gardener.clone()
                        }
                    };

                    // text_range is local coordicate because of clone_subtree()
                    let mut range: std::ops::Range<usize> = common_anscestor.node.text_range().into();
                    (range.start, range.end) = (range.start + global_byte_offset, range.end + global_byte_offset);
                    
                    let strategy = common_anscestor.pick_terminate_kind(stmt_scanner.scanner_type(), self.engine.parsing_rules);

                    let scanner_view = stmt_scanner.as_view(range.start..);
                    let old_metadata_map = common_anscestor.metadata_entry;
                    let metadata = old_metadata_map.map
                        .get(&&NodeMetadataKey::from_raw_node(&common_anscestor.node, self.engine.parsing_rules))
                        .expect("All node have metadata")
                    ;

                    let (new_node, new_matadata_map) = parse_internal(scanner_view, &self.config, metadata.edit_state, strategy, self.engine.parsing_rules)?;
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
                SlotEvent::Inserting { scanner: stmt_scanner } => {
                    let scanner_view = stmt_scanner.as_view(std::ops::RangeFull);
                    let strategy = IncrementalParserStrategy::default_strategy(self.engine.parsing_rules);

                    let (new_stmt, new_matadata_map) = parse_internal(scanner_view, &self.config, 0, strategy, self.engine.parsing_rules)?;
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
                SlotEvent::Deleting{ .. } => {
                    // no parse action
                    // Note that it does not consume iterator
                    continue;
                }
            };

            global_byte_offset += usize::from(new_stmt.text_len()); 

            fragments.push(SyntaxFragment::new(fragments.len(), new_stmt, new_metadata_entry, new_node_key))
        }

        let replace_size = slots.events.into_iter()
            .filter(|slot| matches!(slot, SlotEvent::Replacing{..} | SlotEvent::Deleting{..}))
            .count()
        ;

        let batch = SyntaxFragmentBatch{
            fragments,
            replace_from: slots.replace_from,
            replace_size,
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
    /// new editing text
    pub text: String,
}

impl EditScope {
    pub fn adjust_offset(&self, offset: usize) -> EditScope {
        let offset = usize::max(self.start_char_offset, offset);
        Self {
            start_char_offset: offset,
            old_char_len: self.old_char_len + offset - self.start_char_offset,
            new_char_len: self.new_char_len + offset - self.start_char_offset,
            text: self.text.clone(),
        }
    }

    pub fn old_char_range(&self) -> std::ops::Range<usize> {
        std::ops::Range { start: self.start_char_offset, end: self.start_char_offset + self.old_char_len }
    }

    pub fn new_char_range(&self) -> std::ops::Range<usize> {
        std::ops::Range { start: self.start_char_offset, end: self.start_char_offset + self.new_char_len }
    }
}

fn convert_from_utf16_to_byte_offset(char_len: usize, source: &str) -> Option<usize> {
    let mut current_char_offset = 0;
    let mut last_byte_offset = 0;

    for (byte_offset, ch) in source.char_indices() {
        if current_char_offset == char_len { return Some(byte_offset); }

        current_char_offset += ch.len_utf16();
        // if over current_char_offset, middle of surrogate pair
        if current_char_offset > char_len { return Some(byte_offset); }
        last_byte_offset = byte_offset;
    }

    Some(last_byte_offset)
}

fn convert_from_utf16_to_byte_range(char_range: std::ops::Range<usize>, source: &str) -> std::ops::Range<usize> {
    if let Some(from_offset) = convert_from_utf16_to_byte_offset(char_range.start, source) {
        return match convert_from_utf16_to_byte_offset(char_range.len(), &source[from_offset..]) {
            Some(to_len) => from_offset..(from_offset + to_len),
            None => from_offset..source.len()
        }
    }

    0..source.len()
}

fn parse_internal(
    mut scanner: StatementScannerView, 
    config: &ParserConfig,
    edit_state: usize,
    parse_strategy: impl parser_core::parser::ParseStrategy,
    engine: ParsingRuleSet) -> Result<(rowan::GreenNode, HashMap<NodeMetadataKey, NodeMetadata>), parser_core::parser::ParseError> 
{
    let mut dispatcher = ParseEventDispatcher::new(edit_state, config.mode.clone(), engine);
    let mut tree_builder = SyntaxTreeBuilder::new(engine, config.mode.clone(), None);

    match super::parser::parse_with_config_internal(&mut scanner, &mut dispatcher, &mut tree_builder, config, engine, parse_strategy) {
        Ok(_) => {
            Ok(tree_builder.build_branch()?)
        }
        Err(ParseError::ByEvent(parser_core::event_dispatcher::ParseEventError::NoMoreState{..})) => {
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
