use std::rc::Rc;
use crate::core::engine_core::{parser_engine::ParsingRuleSet, SyntaxKind};
use crate::core::parser_core::metadata::MetadataTable;
use crate::core::parser_core::{NodeMetadata, NodeMetadataKey, ParseMode};

mod tree;
mod node;
mod token;
mod syntax_batch;

pub use tree::SyntaxTree;
pub use node::SyntaxNode;
pub use token::{SyntaxTokenSet, SyntaxTokenItem, SyntaxTokenItems};
pub use syntax_batch::{SyntaxFragment, FragmentNodeMetadataKey, SyntaxFragmentBatch, ApplyBatch, FragmentNodeIterator, FragmentNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowanLangageImpl;

impl rowan::Language for RowanLangageImpl {
    type Kind = u32;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        raw.0 as u32
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind as u16)
    }
}

pub trait MetadataAccess {
    fn metadata_key(&self) -> NodeMetadataKey;
    fn metadata(&self) -> NodeMetadata;
}

pub trait NodeOperation {
    type Item;
    type Parent;
    
    fn parent(&self) -> Option<Self::Parent>;
    fn prev_sibling(&self) -> Option<Self::Item>;
    fn next_sibling(&self) -> Option<Self::Item>;
}

pub trait LookupCandidate {
    fn lookup_candidates(&self) -> impl Iterator<Item = SyntaxKind>;
}

#[derive(Clone, Debug)]
pub(crate) struct SyntaxNodeData {
    raw: rowan::SyntaxNode<RowanLangageImpl>,
    metadata_table: Rc<MetadataTable>,
    parse_mode: ParseMode,
    engine: ParsingRuleSet,
}

impl SyntaxNodeData {
    pub(crate) fn new(
        raw: rowan::SyntaxNode<RowanLangageImpl>, 
        metadata_table: Rc<MetadataTable>,
        parse_mode: ParseMode,
        engine: ParsingRuleSet) -> Self 
    {
        Self {
            raw,
            metadata_table,
            parse_mode,
            engine,
        }
    }

    pub(crate) fn with_raw(&self, raw: &rowan::SyntaxNode<RowanLangageImpl>, parse_mode: ParseMode) -> Self {
        Self {
            raw: raw.clone(),
            metadata_table: self.metadata_table.clone(),
            parse_mode,
            engine: self.engine,
        }
    }

    fn statement_index(&self) -> Option<usize> {
        self.statement_index_raw(&self.raw)
    }

    pub(crate) fn statement_index_raw(&self, raw: &rowan::SyntaxNode<RowanLangageImpl>) -> Option<usize> {
        if self.parse_mode == ParseMode::Full {
            return None;
        }

        let stmt_symbol = self.engine.statement_emit_config().from_symbol;
        raw.ancestors().find_map(|node| {
            let kind = self.engine.from_kind_id(node.kind());
            (kind.id == stmt_symbol.id).then(|| node.index())
        })
    }

    pub(crate) fn metadata_with(&self, index: Option<usize>, key: &NodeMetadataKey) -> NodeMetadata {
        let stmt_metadta = &self.metadata_table.statement_metadata(index);
        let (byte_offset, char_offset ) = if self.parse_mode == ParseMode::Full { (0, 0) } else { (stmt_metadta.global_offset.of_byte, stmt_metadta.global_offset.of_char) };
        let local_key = key.clone().into_local(byte_offset);

        stmt_metadta.map.get(&local_key)
        .expect(&format!("All node/token must contain a metadata@{index:?} (key: {key:?}, byte_offset: {byte_offset})"))
        .into_global(char_offset)
    }
}

impl MetadataAccess for SyntaxNodeData {
    fn metadata_key(&self) -> NodeMetadataKey {
        let range = self.raw.text_range();
        NodeMetadataKey{ 
            kind: self.engine.from_kind_id(self.raw.kind() as u32), 
            offset: range.start().into(), 
            len: range.len().into(), 
            is_leaf: false 
        }
    }
    
    fn metadata(&self) -> NodeMetadata {
        let index = self.statement_index();
        self.metadata_with(index, &self.metadata_key())
    }
}

impl PartialEq for SyntaxNodeData {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw && self.parse_mode == other.parse_mode
    }
}
impl Eq for SyntaxNodeData {}

#[derive(PartialEq, Clone, Debug)]
pub(crate) struct SyntaxTokenData {
    raw: rowan::SyntaxToken<RowanLangageImpl>,
    metadata_table: Rc<MetadataTable>,
    parse_mode: ParseMode,
    engine: ParsingRuleSet,
}

impl SyntaxTokenData {
    fn statement_index(&self) -> Option<usize> {
        if self.parse_mode == ParseMode::Full {
            return None;
        }

        let stmt_symbol = self.engine.statement_emit_config().from_symbol;
        let stmt = self.raw.parent_ancestors().skip_while(|node| {
            let kind = self.engine.from_kind_id(node.kind());
            kind.id != stmt_symbol.id
        })
        .next();

        stmt.map(|node| node.index())
    }
}

impl MetadataAccess for SyntaxTokenData {
    fn metadata_key(&self) -> NodeMetadataKey {
        let range = self.raw.text_range();
        NodeMetadataKey{ 
            kind: self.engine.from_kind_id(self.raw.kind() as u32), 
            offset: range.start().into(), 
            len: range.len().into(), 
            is_leaf: true 
        }
    }

    fn metadata(&self) -> NodeMetadata {
        let index = self.statement_index();
        let stmt_metadta = &self.metadata_table.statement_metadata(index);
        let (byte_offset, char_offset ) = if self.parse_mode == ParseMode::Full { (0, 0) } else { (stmt_metadta.global_offset.of_byte, stmt_metadta.global_offset.of_char) };
        let key = self.metadata_key().into_local(byte_offset);

        stmt_metadta.map.get(&key)
        .expect(&format!("All node/token must contain a metadata@{index:?} (key: {key:?}, byte_offset: {byte_offset})"))
        .into_global(char_offset)
    }
}

impl SyntaxTokenData {
    pub(crate) fn new(
        raw: rowan::SyntaxToken<RowanLangageImpl>, 
        metadata_table: Rc<MetadataTable>,
        parse_mode: ParseMode,
        engine: ParsingRuleSet) -> Self 
    {
        Self {
            raw,
            metadata_table,
            parse_mode,
            engine,
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum SyntaxElementDef<N, S> {
    Node(N),
    TokenSet(S),
}

pub type SyntaxElement = SyntaxElementDef<SyntaxNode, SyntaxTokenSet>;
pub type SyntaxElementRef<'a> = SyntaxElementDef<&'a SyntaxNode, &'a SyntaxTokenSet>;

impl<N, S> SyntaxElementDef<N, S> where N: MetadataAccess, S: MetadataAccess
{
    pub(crate) fn as_accessor(&self) -> &dyn MetadataAccess {
        match self {
            SyntaxElementDef::Node(node) => node as &dyn MetadataAccess,
            SyntaxElementDef::TokenSet(token_set) => token_set as &dyn MetadataAccess,
        }
    }
}

impl<N, S> SyntaxElementDef<N, S> where N: MetadataAccess, S: MetadataAccess {
    pub fn metadata_key(&self) -> NodeMetadataKey {
        self.as_accessor().metadata_key()
    }

    pub fn metadata(&self) -> NodeMetadata {
        self.as_accessor().metadata()
    }
}

impl<N, S> SyntaxElementDef<N, S> where N: Clone, S: Clone {
    pub fn to_node(&self) -> Option<N> {
        match self {
            SyntaxElementDef::Node(node) => Some(node.clone()),
            _ => None,
        }
    }

    pub fn to_token_set(&self) -> Option<S> {
        match self {
            SyntaxElementDef::TokenSet(token_set) => Some(token_set.clone()),
            _ => None,
        }
    }
}

impl NodeOperation for SyntaxElement {
    type Item = SyntaxElementDef<SyntaxNode, SyntaxTokenSet>;
    type Parent = SyntaxNode;

    fn parent(&self) -> Option<Self::Parent> {
        match self {
            SyntaxElementDef::Node(node) => node.parent(),
            SyntaxElementDef::TokenSet(token_set) => token_set.parent(),
        }
    }

    fn prev_sibling(&self) -> Option<Self::Item> {
        match self {
            SyntaxElementDef::Node(node) => node.prev_sibling(),
            SyntaxElementDef::TokenSet(token_set) => token_set.prev_sibling(),
        }
    }

    fn next_sibling(&self) -> Option<Self::Item> {
        match self {
            SyntaxElementDef::Node(node) => node.next_sibling(),
            SyntaxElementDef::TokenSet(token_set) => token_set.next_sibling(),
        }
    }
}
