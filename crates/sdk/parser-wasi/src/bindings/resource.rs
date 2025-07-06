use tolerant_parser_sdk::core::parser_core::{self, ParserConfig, syntax_tree::{ApplyBatch, LookupCandidate, MetadataAccess, NodeOperation, SyntaxTree}};
use tolerant_parser_sdk::core::engine_core;
use super::parser_world::exports::ritalin::parser::parsers;
use super::syntax_tree_world::exports::ritalin::parser::syntaxes;
use super::types_world::exports::ritalin::parser::types;

pub struct ParserImpl {
    inner: parser_core::Parser,
}

impl ParserImpl {
    pub fn new(engine: engine_core::Engine, config: ParserConfig) -> Self {
        Self {
            inner: parser_core::Parser::new(engine, config),
        }
    }
}

impl parsers::GuestParser for ParserImpl {
    fn parse(&self,source: String,) -> parsers::SyntaxTree {
        let tree = self.inner.parse(&source).expect("Failed to parse");
        SyntaxTreeImpl::from_raw(tree)
    }
    
    fn incremental(&self,tree: parsers::SyntaxTree,scopes: Vec::<parsers::EditScope>,) -> parsers::IncrementalParser {
        let old_tree = tree.into_inner::<SyntaxTreeImpl>().inner;

        let scopes: Vec<parser_core::incremental::EditScope> = scopes.into_iter()
            .map(Into::into)
            .collect::<Vec<_>>()
        ;

        IncrementalParserImpl::from_raw(
            self.inner.incremental(&old_tree, scopes.first().unwrap().clone()),
            old_tree
        )
    }
}

pub struct IncrementalParserImpl {
    inner: parser_core::incremental::Parser,
    old_tree: SyntaxTree,
}

impl IncrementalParserImpl {
    pub(crate) fn from_raw(inner: parser_core::incremental::Parser, old_tree: SyntaxTree) -> parsers::IncrementalParser {
        parsers::IncrementalParser::new(Self { inner, old_tree })
    }
}

impl parsers::GuestIncrementalParser for IncrementalParserImpl {
    fn parse(&self,source: String,) -> parsers::SyntaxTree {
        let batches = self.inner.parse(&source).expect("Failed to parse");

        let new_tree = self.old_tree.apply_batches(batches);
        SyntaxTreeImpl::from_raw(new_tree)
    }
}

pub struct SyntaxTreeImpl {
    inner: parser_core::syntax_tree::SyntaxTree,
}

impl SyntaxTreeImpl {
    pub fn from_raw(tree: parser_core::syntax_tree::SyntaxTree) -> syntaxes::SyntaxTree {
        syntaxes::SyntaxTree::new(Self { inner: tree })
    }
}

impl syntaxes::GuestSyntaxTree for SyntaxTreeImpl {
    fn root(&self,) -> syntaxes::SyntaxNode {
        SyntaxNodeImpl::from_raw(self.inner.root().clone())
    }
}

pub struct SyntaxNodeImpl {
    inner: parser_core::syntax_tree::SyntaxNode,
}

impl SyntaxNodeImpl {
    pub fn from_raw(node: parser_core::syntax_tree::SyntaxNode) -> syntaxes::SyntaxNode {
        syntaxes::SyntaxNode::new(Self { inner: node })
    }
}

impl syntaxes::GuestSyntaxNode for SyntaxNodeImpl {
    fn metadata_key(&self,) -> syntaxes::MetadataKey {
        self.inner.metadata_key().into()
    }

    fn metadata(&self,) -> syntaxes::Metadata {
        self.inner.metadata().into()
    }

    fn parent(&self,) -> Option<syntaxes::SyntaxNode> {
        self.inner.parent().as_ref().map(|node| SyntaxNodeImpl::from_raw(node.clone()))
    }

    fn children(&self,) -> Vec::<syntaxes::SyntaxElement> {
        self.inner.children()
        .map(|node| node.clone().into())
        .collect()
    }
    
    fn token_at_offset(&self,char_offset: u32,) -> Option<syntaxes::SyntaxTokenItem> {
        self.inner.token_at_utf16_offset(char_offset as usize)
        .map(|item| SyntaxTokenItemImpl::from_raw(item))
    }
    
    fn prev_sibling(&self,) -> Option<syntaxes::SyntaxElement> {
        self.inner.prev_sibling() 
        .map(|el| el.into())
    }
    
    fn next_sibling(&self,) -> Option<syntaxes::SyntaxElement> {
        self.inner.next_sibling() 
        .map(|el| el.into())
    }
    
    fn descendant_nodes(&self,) -> Vec::<syntaxes::SyntaxElement> {
        self.inner.descendant_nodes()
        .map(|el| el.into())
        .collect()
    }
}

pub struct SyntaxTokenSetImpl {
    inner: parser_core::syntax_tree::SyntaxTokenSet,
}

impl SyntaxTokenSetImpl {
    pub fn from_raw(token_set: parser_core::syntax_tree::SyntaxTokenSet) -> syntaxes::SyntaxTokenSet {
        syntaxes::SyntaxTokenSet::new(Self { inner: token_set })
    }
}

impl syntaxes::GuestSyntaxTokenSet for SyntaxTokenSetImpl {
    fn metadata_key(&self,) -> syntaxes::MetadataKey {
        self.inner.metadata_key().into()
    }

    fn metadata(&self,) -> syntaxes::Metadata {
        self.inner.metadata().into()
    }

    fn parent(&self,) -> Option<syntaxes::SyntaxNode> {
        self.inner.parent().as_ref().map(|node| SyntaxNodeImpl::from_raw(node.clone()))
    }

    fn leading_trivia(&self,) -> Vec::<syntaxes::SyntaxTokenItem> {
        self.inner.leading_trivia()
        .map(|node| SyntaxTokenItemImpl::from_raw(node))
        .collect()
    }

    fn token(&self,) -> syntaxes::SyntaxTokenItem {
        SyntaxTokenItemImpl::from_raw(self.inner.token())
    }

    fn trailing_trivia(&self,) -> Vec::<syntaxes::SyntaxTokenItem> {
        self.inner.trailing_trivia()
        .map(|node| SyntaxTokenItemImpl::from_raw(node))
        .collect()
    }
    
    fn prev_sibling(&self,) -> Option<syntaxes::SyntaxElement> {
        self.inner.prev_sibling() 
        .map(|el| el.into())
    }
    
    fn next_sibling(&self,) -> Option<syntaxes::SyntaxElement> {
        self.inner.next_sibling() 
        .map(|el| el.into())
    }
    
    fn lookup_candidates(&self,) -> Vec::<syntaxes::SyntaxKind> {
        self.inner.lookup_candidates().into_iter()
        .map(Into::into)
        .collect()
    }
    
    fn descendant_tokens(&self,) -> Vec::<syntaxes::SyntaxTokenItem> {
        self.inner.descendant_tokens()
        .map(|node| SyntaxTokenItemImpl::from_raw(node))
        .collect()
    }
}

pub struct SyntaxTokenItemImpl {
    inner: parser_core::syntax_tree::SyntaxTokenItem,
}

impl SyntaxTokenItemImpl {
    pub fn from_raw(item: parser_core::syntax_tree::SyntaxTokenItem) -> syntaxes::SyntaxTokenItem {
        syntaxes::SyntaxTokenItem::new(Self { inner: item })
    }
}

impl syntaxes::GuestSyntaxTokenItem for SyntaxTokenItemImpl {
    fn metadata_key(&self,) -> syntaxes::MetadataKey {
        self.inner.metadata_key().into()
    }

    fn metadata(&self,) -> syntaxes::Metadata {
        self.inner.metadata().into()
    }

    fn parent(&self,) -> Option<syntaxes::SyntaxTokenSet> {
        self.inner.parent().as_ref().map(|node| SyntaxTokenSetImpl::from_raw(node.clone()))
    }

    fn value(&self,) -> String {
        self.inner.value().into()
    }
    
    fn prev_token(&self,) -> Option<syntaxes::SyntaxTokenItem> {
        self.inner.prev_sibling()
        .map(|item| SyntaxTokenItemImpl::from_raw(item))
    }
    
    fn next_token(&self,) -> Option<syntaxes::SyntaxTokenItem> {
        self.inner.next_sibling()
        .map(|item| SyntaxTokenItemImpl::from_raw(item))
    }
}

impl From<parser_core::syntax_tree::SyntaxElement> for syntaxes::SyntaxElement {
    fn from(value: parser_core::syntax_tree::SyntaxElement) -> Self {
        match value {
            parser_core::syntax_tree::SyntaxElementDef::Node(node) => {
                syntaxes::SyntaxElement::Node(SyntaxNodeImpl::from_raw(node))
            }
            parser_core::syntax_tree::SyntaxElementDef::TokenSet(token_set) => {
                syntaxes::SyntaxElement::TokenSet(SyntaxTokenSetImpl::from_raw(token_set))
            }
        }
    }
}

impl From<parser_core::NodeMetadataKey> for syntaxes::MetadataKey {
    fn from(value: parser_core::NodeMetadataKey) -> Self {
        Self {
            kind: value.kind.into(),
            offset: value.offset as u32,
            len: value.len as u32,
            is_leaf: value.is_leaf,
        }
    }
}

impl From<parser_core::NodeMetadata> for syntaxes::Metadata {
    fn from(value: parser_core::NodeMetadata) -> Self {
        Self {
            node_type: value.node_type.clone().into(),
            edit_state: value.edit_state as u64,
            patch: value.patch.clone().into(),
            char_offset: value.char_offset as u32,
            char_len: value.char_len as u32,
        }
    }
}

impl From<engine_core::SyntaxKind> for types::SyntaxKind {
    fn from(value: engine_core::SyntaxKind) -> Self {
        Self {
            name: value.text.into(),
            group: value.group.into(),
        }
    }
}

impl From<engine_core::SymbolGroup> for types::SymbolGroup {
    fn from(value: engine_core::SymbolGroup) -> Self {
        match value {
            engine_core::SymbolGroup::Keyword => types::SymbolGroup::Keyword,
            engine_core::SymbolGroup::NonKeyword => types::SymbolGroup::NonKeyword,
            engine_core::SymbolGroup::Pattern => types::SymbolGroup::Pattern,
            engine_core::SymbolGroup::NonTerminal => types::SymbolGroup::NonTerminal,
        }
    }
}

impl From<parser_core::NodeType> for types::NodeType {
    fn from(value: parser_core::NodeType) -> Self {
        match value {
            parser_core::NodeType::Node => types::NodeType::Node,
            parser_core::NodeType::TokenSet => types::NodeType::TokenSet,
            parser_core::NodeType::TokenItem => types::NodeType::TokenItem,
            parser_core::NodeType::LeadingToken => types::NodeType::LeadingTrivia,
            parser_core::NodeType::TrailingToken => types::NodeType::TrailingTrivia,
        }
    }
}

impl From<parser_core::PatchAction> for types::PatchAction {
    fn from(value: parser_core::PatchAction) -> Self {
        match value {
            parser_core::PatchAction::None => types::PatchAction::None,
            parser_core::PatchAction::Delete => types::PatchAction::Delete,
            parser_core::PatchAction::Shift => types::PatchAction::Shift,
            parser_core::PatchAction::Invalid => types::PatchAction::Invalid,
        }
    }
}

impl From<parsers::EditScope> for parser_core::incremental::EditScope {
    fn from(value: parsers::EditScope) -> Self {
        Self {
            start_char_offset: value.start_offset as usize,
            old_char_len: value.old_len as usize,
            new_char_len: value.new_len as usize,
        }
    }
}

pub struct SyntaxTreeComponent;

impl syntaxes::Guest for SyntaxTreeComponent {
    type SyntaxTree = SyntaxTreeImpl;
    type SyntaxNode = SyntaxNodeImpl;
    type SyntaxTokenSet = SyntaxTokenSetImpl;
    type SyntaxTokenItem = SyntaxTokenItemImpl;
}

#[allow(unused)]
pub enum ParserTypesComponent {}