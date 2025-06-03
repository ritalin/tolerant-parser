use std::rc::Rc;
use engine_core::{parser_engine::ParsingRuleSet, SyntaxKind};
use scanner_core::{Scanner, StatementScannerView};
use crate::{metadata::StatementMetadataMap, node_handler::SyntaxTreeBuilder, syntax_tree::{RowanLangageImpl, SyntaxTree}, ParserConfig};

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
        let scanner = Scanner::create(source, self.scope.start_byte_offset as u32, self.engine.scanning_rules.clone())?;
        let terminate_symbol = self.engine.parsing_rules.statement_emit_config().to_symbol;

        let mut new_children = vec![];
        let mut new_metadata_table = vec![];
        let stmts = self.statements.iter().map(Some).chain(std::iter::repeat(None));

        for (stmt_scanner, stmt) in scanner.statement_scanners(terminate_symbol).zip(stmts) {
            match stmt {
                Some(stmt) => {
                    let stmt_index = stmt.index();
                    let stmt = stmt.clone_subtree();
                    let gardener = support::TreeGardener{ stmt_node: stmt.clone() };
                    let (lowest, highest) = self.scope.adjust_scope(stmt);
                    // Find common anscestor
                    let common_anscestor = 
                        gardener.common_anscestor(
                            gardener.left_hand_token_for(lowest.into()), 
                            gardener.left_hand_token_for(highest.into()),
                            terminate_symbol
                        )
                        .expect("At least, must exist")
                    ;
                    let anscestor_range: std::ops::Range<usize> = common_anscestor.text_range().into();
                    let stmt_scanner = stmt_scanner.as_view(anscestor_range.start..(anscestor_range.end + 1));
                    let old_metadata_map = self.metadata_table.get(stmt_index);
                    let (new_node, new_matadata_map) = parse_internal(stmt_scanner, &config, self.engine.parsing_rules)?;
                    
                    new_children.push(gardener.replace_with_new_node(new_node, &common_anscestor));
                    new_metadata_table.push(gardener.merge_metadata_map(common_anscestor, old_metadata_map, new_matadata_map));
                }
                None => todo!(),
            }
        }
        
        let root = self.root.green().splice_children(self.replace_from..(self.replace_from + self.statements.len()), new_children);
        let metadata_table = update_metadata_table(root.children(), self.metadata_table.as_ref(), new_metadata_table, self.replace_from, self.statements.len());

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
    pub fn adjust_scope(&self, node: rowan::SyntaxNode<RowanLangageImpl>) -> (u32, u32) {
        let range = node.text_range();
        let lowest_offset = 
            u32::max(self.start_byte_offset as u32, range.start().into())
        ;
        let highest_offset = 
            u32::min(
                (self.start_byte_offset + usize::max(self.old_byte_len, self.new_byte_len)) as u32, 
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
    engine: ParsingRuleSet) -> Result<(rowan::GreenNode, StatementMetadataMap), crate::parser::ParseError> 
{
    let mut tree_builder = SyntaxTreeBuilder::new(engine, config.mode.clone(), None);
    let strategy = IncrementalParserStrategy{ terminate_kind: engine.statement_emit_config().to_symbol };

    let event = super::parser::parse_with_config_internal(&mut scanner, &mut tree_builder, config, engine, strategy)?;

    todo!()
}

fn update_metadata_table<'a>(
    children: impl Iterator<Item = rowan::NodeOrToken<&'a rowan::GreenNodeData, &'a rowan::GreenTokenData>>,
    old_table: &[StatementMetadataMap], 
    changed_maps: Vec<StatementMetadataMap>, 
    replace_from: usize, old_len: usize) -> Vec<StatementMetadataMap> 
{
    let mut metadata_table = Vec::from_iter(old_table.iter().cloned());
    metadata_table.splice(replace_from..(replace_from + old_len), changed_maps);

    // update offset

    todo!()
}