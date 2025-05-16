use std::{collections::HashMap, rc::Rc};
use engine_core::parser_engine::ParsingRuleSet;
use rowan::NodeOrToken;
use crate::{node_handler::{NodeMetadata, NodeMetadataKey}, NodeId};

#[derive(PartialEq, Debug)]
pub struct SyntaxTree {
    root: rowan::SyntaxNode<RowanLangageImpl>,
    metadata_map: Rc<HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>>,
    engine: ParsingRuleSet,
}

impl SyntaxTree {
    pub fn root(&self) -> SyntaxNode {
        SyntaxNode::new(self.root.clone(), self.metadata_map.clone(), self.engine)
    }
}

impl SyntaxTree {
    pub (crate) fn new(
        root: rowan::GreenNode, 
        metadata_map: HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>,
        engine: ParsingRuleSet) -> Self 
    {
        Self {
            root: rowan::api::SyntaxNode::new_root_mut(root),
            metadata_map: Rc::new(metadata_map),
            engine,
        }
    }
}

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

#[derive(PartialEq, Debug)]
pub enum NodeType {
    Node,
    TokenSet,
    TokenItem,
    LeadingToken,
    TrailingToken,
    Error,
}

pub trait MetadataAccess {
    fn metadata_key(&self) -> NodeMetadataKey;
    fn metadata(&self) -> &NodeMetadata;
}

#[derive(Clone)]
pub struct SyntaxNode {
    raw: rowan::SyntaxNode<RowanLangageImpl>,
    metadata_map: Rc<HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>>,
    engine: ParsingRuleSet,
}

impl SyntaxNode {
    pub fn cursor<'a>(&'a self) -> NodeCursor {
        NodeCursor::new(self.raw.clone(), self.metadata_map.clone(), self.engine)
    }

    pub fn children(&self) -> SyntaxNodeChildren {
        SyntaxNodeChildren::new(self)
    }
}

impl MetadataAccess for SyntaxNode {
    fn metadata_key(&self) -> NodeMetadataKey {
        NodeMetadataKey::from_node(&self.raw, self.engine)
    }

    fn metadata(&self) -> &NodeMetadata {
        self.metadata_map.get(&self.metadata_key()).map(|(_, metadata)| metadata).expect("All node/token must contain a metadata")
    }
}

impl SyntaxNode {
    pub(crate) fn new(
        raw: rowan::SyntaxNode<RowanLangageImpl>, 
        metadata_map: Rc<HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>>,
        engine: ParsingRuleSet) -> Self {
        Self { raw, metadata_map, engine }
    }
}

pub struct NodeCursor {
    raw: rowan::SyntaxNode<RowanLangageImpl>,
    metadata_map: Rc<HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>>,
    engine: ParsingRuleSet
}

impl NodeCursor {
    pub fn nth_child(&self, index: usize) -> Option<SyntaxElement> {
        let child = match index {
            0 => self.raw.first_child_or_token(),
            _ => self.raw.children_with_tokens().nth(index)
        };

        match child.as_ref() {
            Some(NodeOrToken::Node(node)) => {
                let key = NodeMetadataKey::from_node(node, self.engine);
                match self.metadata_map.get(&key) {
                    Some((_, metadata)) if metadata.node_type == NodeType::TokenSet => {
                        Some(SyntaxElement::TokenSet(SyntaxTokenSet::new(node.clone(), self.metadata_map.clone(), self.engine)))
                    }
                    Some(_) => {
                        Some(SyntaxElement::Node(SyntaxNode::new(node.clone(), self.metadata_map.clone(), self.engine)))
                    }
                    None => None
                }
            }
            Some(NodeOrToken::Token(node)) => {
                Some(SyntaxElement::TokenItem(SyntaxTokenItem::new(node.clone(), self.metadata_map.clone(), self.engine)))
            }
            None => None,
        }
    }
}

impl NodeCursor {
    pub(crate) fn new(
        raw: rowan::SyntaxNode<RowanLangageImpl>, 
        metadata_map: Rc<HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>>,
        engine: ParsingRuleSet) -> Self 
    {
        Self { raw, metadata_map, engine }
    }
}

pub struct SyntaxNodeChildren {
    raw: rowan::SyntaxElementChildren<RowanLangageImpl>,
    metadata_map: Rc<HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>>,
    engine: ParsingRuleSet
}

impl SyntaxNodeChildren {
    pub(crate) fn new(node: &SyntaxNode) -> Self {
        Self { raw: node.raw.children_with_tokens(), metadata_map: node.metadata_map.clone(), engine: node.engine }
    }
}

impl Iterator for SyntaxNodeChildren {
    type Item = SyntaxElement;

    fn next(&mut self) -> Option<Self::Item> {
        match self.raw.next() {
            Some(rowan::SyntaxElement::Node(node)) => {
                let key = NodeMetadataKey::from_node(&node, self.engine);
                match self.metadata_map.get(&key) {
                    Some((_, metadata)) if metadata.node_type == NodeType::Node => {
                        Some(SyntaxElement::Node(SyntaxNode::new(node, self.metadata_map.clone(), self.engine)))
                    }
                    Some(_) => {
                        Some(SyntaxElement::TokenSet(SyntaxTokenSet::new(node, self.metadata_map.clone(), self.engine)))
                    }
                    None => None
                }
            }
            Some(rowan::SyntaxElement::Token(node)) => {
                Some(SyntaxElement::TokenItem(SyntaxTokenItem::new(node, self.metadata_map.clone(), self.engine)))
            }
            None => None
        }
    }
}

#[derive(Clone)]
pub struct SyntaxTokenSet {
    raw: rowan::SyntaxNode<RowanLangageImpl>,
    metadata_map: Rc<HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>>,
    engine: ParsingRuleSet,
}

impl SyntaxTokenSet {
    pub fn leading_trivia(&self) -> SyntaxTriviaItems {
        SyntaxTriviaItems::new(self, NodeType::LeadingToken)
    }
    pub fn trailing_trivia(&self) -> SyntaxTriviaItems {
        SyntaxTriviaItems::new(self, NodeType::TrailingToken)
    }
    pub fn token(&self) -> SyntaxTokenItem {
        SyntaxTriviaItems::new(self, NodeType::TokenItem).next().expect("Missing Main token item in token set")
    }
}

impl MetadataAccess for SyntaxTokenSet {
    fn metadata_key(&self) -> NodeMetadataKey {
        NodeMetadataKey::from_node(&self.raw, self.engine)
    }

    fn metadata(&self) -> &NodeMetadata {
        self.metadata_map.get(&self.metadata_key()).map(|(_, metadata)| metadata).expect("All node/token must contain a metadata")
    }
}

impl SyntaxTokenSet {
    pub(crate) fn new(
        raw: rowan::SyntaxNode<RowanLangageImpl>, 
        metadata_map: Rc<HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>>,
        engine: ParsingRuleSet) -> Self 
    {
        Self { raw, metadata_map, engine }
    }
}

#[derive(Clone)]
pub struct SyntaxTokenItem {
    raw: rowan::SyntaxToken<RowanLangageImpl>,
    metadata_map: Rc<HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>>,
    engine: ParsingRuleSet,
}

impl SyntaxTokenItem {
    pub fn value<'a>(&'a self) -> &'a str {
        self.raw.text()
    }
}

impl SyntaxTokenItem {
    pub(crate) fn new(
        raw: rowan::SyntaxToken<RowanLangageImpl>,
        metadata_map: Rc<HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>>,
        engine: ParsingRuleSet) -> Self 
    {
        Self { raw, metadata_map, engine }
    }
}

impl MetadataAccess for SyntaxTokenItem {
    fn metadata_key(&self) -> NodeMetadataKey {
        NodeMetadataKey::from_token(&self.raw, self.engine)
    }

    fn metadata(&self) -> &NodeMetadata {
        self.metadata_map.get(&self.metadata_key()).map(|(_, metadata)| metadata).expect("All node/token must contain a metadata")
    }
}

pub struct SyntaxTriviaItems {
    children: rowan::SyntaxElementChildren<RowanLangageImpl>,
    metadata_map: Rc<HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>>,
    engine: ParsingRuleSet,
    node_type: NodeType,
}

impl Iterator for SyntaxTriviaItems {
    type Item = SyntaxTokenItem;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(rowan::NodeOrToken::Token(item)) = self.children.next() {
            let key = NodeMetadataKey::from_token(&item, self.engine);
            match self.metadata_map.get(&key) {
                Some((_, metadata)) if metadata.node_type == self.node_type => {
                    return Some(SyntaxTokenItem::new(item.clone(), self.metadata_map.clone(), self.engine));
                }
                _ => {}
            }
        }

        None
    }
}

impl SyntaxTriviaItems {
    pub(crate) fn new(item: &SyntaxTokenSet, node_type: NodeType) -> Self {
        Self {
            children: item.raw.children_with_tokens(),
            metadata_map: item.metadata_map.clone(),
            engine: item.engine,
            node_type,
        }
    }
}

pub enum SyntaxElementDef<N, S, I> {
    Node(N),
    TokenSet(S),
    TokenItem(I),
}

pub type SyntaxElement = SyntaxElementDef<SyntaxNode, SyntaxTokenSet, SyntaxTokenItem>;
pub type SyntaxElementRef<'a> = SyntaxElementDef<&'a SyntaxNode, &'a SyntaxTokenSet, &'a SyntaxTokenItem>;

impl<N, S, I> SyntaxElementDef<N, S, I> where N: MetadataAccess, S: MetadataAccess, I: MetadataAccess
{
    pub(crate) fn as_accessor(&self) -> &dyn MetadataAccess {
        match self {
            SyntaxElementDef::Node(node) => node as &dyn MetadataAccess,
            SyntaxElementDef::TokenSet(token_set) => token_set as &dyn MetadataAccess,
            SyntaxElementDef::TokenItem(token_item) => token_item as &dyn MetadataAccess,
        }
    }
}

impl<N, S, I> SyntaxElementDef<N, S, I> where N: MetadataAccess, S: MetadataAccess, I: MetadataAccess {
    pub fn metadata_key(&self) -> NodeMetadataKey {
        self.as_accessor().metadata_key()
    }

    pub fn metadata(&self) -> &NodeMetadata {
        self.as_accessor().metadata()
    }
}

impl<N, S, I> SyntaxElementDef<N, S, I> where N: Clone, S: Clone, I: Clone {
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

    pub fn to_token_item(&self) -> Option<I> {
        match self {
            SyntaxElementDef::TokenItem(token_item) => Some(token_item.clone()),
            _ => None,
        }
    }
}