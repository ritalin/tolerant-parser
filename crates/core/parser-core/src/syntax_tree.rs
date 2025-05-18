use std::{collections::HashMap, rc::Rc};
use engine_core::parser_engine::ParsingRuleSet;
use crate::{NodeId, NodeMetadata, NodeMetadataKey};

mod tree;
mod node;
mod token;

pub use tree::SyntaxTree;
pub use node::SyntaxNode;
pub use token::{SyntaxTokenSet, SyntaxTokenItem, SyntaxTokenItems};

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
    fn metadata(&self) -> &NodeMetadata;
}

pub trait NodeOperation {
    type Item;
    
    fn parent(&self) -> Option<SyntaxNode>;
    fn prev_sibling(&self) -> Option<Self::Item>;
    fn next_sibling(&self) -> Option<Self::Item>;
}

#[derive(Clone, Debug)]
pub(crate) struct SyntaxNodeData {
    raw: rowan::SyntaxNode<RowanLangageImpl>,
    metadata_map: Rc<HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>>,
    engine: ParsingRuleSet,
}

impl SyntaxNodeData {
    pub(crate) fn new(
        raw: rowan::SyntaxNode<RowanLangageImpl>, 
        metadata_map: Rc<HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>>,
        engine: ParsingRuleSet) -> Self 
    {
        Self {
            raw,
            metadata_map,
            engine,
        }
    }

    pub(crate) fn with_raw(&self, raw: &rowan::SyntaxNode<RowanLangageImpl>) -> Self {
        Self {
            raw: raw.clone(),
            metadata_map: self.metadata_map.clone(),
            engine: self.engine,
        }
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
    
    fn metadata(&self) -> &NodeMetadata {
        let key = self.metadata_key();
        self.metadata_map.get(&key)
        .map(|(_, metadata)| metadata)
        .expect(&format!("All node/token must contain a metadata (key: {key:?})"))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SyntaxTokenData {
    raw: rowan::SyntaxToken<RowanLangageImpl>,
    metadata_map: Rc<HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>>,
    engine: ParsingRuleSet,
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

    fn metadata(&self) -> &NodeMetadata {
        self.metadata_map.get(&self.metadata_key())
        .map(|(_, metadata)| metadata)
        .expect("All node/token must contain a metadata")
    }
}

impl SyntaxTokenData {
    pub(crate) fn new(
        raw: rowan::SyntaxToken<RowanLangageImpl>, 
        metadata_map: Rc<HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>>,
        engine: ParsingRuleSet) -> Self 
    {
        Self {
            raw,
            metadata_map,
            engine,
        }
    }
}

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

    pub fn metadata(&self) -> &NodeMetadata {
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
