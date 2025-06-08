use std::rc::Rc;
use engine_core::parser_engine::ParsingRuleSet;
use crate::{metadata::StatementMetadataMap, syntax_tree::LookupCandidate, NodeMetadata, NodeMetadataKey, NodeType, ParseMode};
use super::{MetadataAccess, NodeOperation, RowanLangageImpl, SyntaxNode, SyntaxNodeData, SyntaxTokenData};

#[derive(PartialEq, Clone, Debug)]
pub struct SyntaxTokenSet {
    data: SyntaxNodeData,
}

impl SyntaxTokenSet {
    pub fn leading_trivia(&self) -> SyntaxTokenItems {
        SyntaxTokenItems::new(&self.data, NodeType::LeadingToken)
    }
    pub fn trailing_trivia(&self) -> SyntaxTokenItems {
        SyntaxTokenItems::new(&self.data, NodeType::TrailingToken)
    }
    pub fn token(&self) -> SyntaxTokenItem {
        SyntaxTokenItems::new(&self.data, NodeType::TokenItem).next().expect("Missing Main token item in token set")
    }
}

impl MetadataAccess for SyntaxTokenSet {
    fn metadata_key(&self) -> NodeMetadataKey {
        self.data.metadata_key()
    }

    fn metadata(&self) -> NodeMetadata {
        self.data.metadata()
    }
}

impl NodeOperation for SyntaxTokenSet {
    type Item = super::SyntaxElement;
    type Parent = SyntaxNode;

    fn parent(&self) -> Option<Self::Parent> {
        match self.data.raw.parent() {
            Some(node) => {
                Some(SyntaxNode::from_raw(self.data.with_raw(&node, self.data.parse_mode.clone())))
            }
            None => None
        }
    }

    fn prev_sibling(&self) -> Option<Self::Item> {
        match self.data.raw.prev_sibling() {
            Some(node) => {
                let data = SyntaxNodeData::new(
                    node, 
                    self.data.metadata_table.clone(), 
                    self.data.parse_mode.clone(), 
                    self.data.engine
                );
                match data.metadata().node_type {
                    NodeType::TokenSet => Some(super::SyntaxElement::TokenSet(SyntaxTokenSet::from_raw(data))),
                    NodeType::Node => Some(super::SyntaxElementDef::Node(SyntaxNode::from_raw(data))),
                    _ => None
                }

            }
            None => None,
        }
    }

    fn next_sibling(&self) -> Option<Self::Item> {
        match self.data.raw.next_sibling() {
            Some(node) => {
                let data = SyntaxNodeData::new(
                    node, 
                    self.data.metadata_table.clone(), 
                    self.data.parse_mode.clone(), 
                    self.data.engine
                );
                match data.metadata().node_type {
                    NodeType::TokenSet => Some(super::SyntaxElement::TokenSet(SyntaxTokenSet::from_raw(data))),
                    NodeType::Node => Some(super::SyntaxElementDef::Node(SyntaxNode::from_raw(data))),
                    _ => None
                }

            }
            None => None,
        }    }
}

impl LookupCandidate for SyntaxTokenSet {
    fn lookup_candidates(&self) -> impl Iterator<Item = engine_core::SyntaxKind> {
        let metadata = self.metadata();

        self.data.engine.candidate_terminal_symbols(metadata.edit_state).into_iter()
        .map(Clone::clone)
    }
}

impl SyntaxTokenSet {
    pub(crate) fn from_raw(data: SyntaxNodeData) -> Self {
        Self { data }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct SyntaxTokenItem {
    data: SyntaxTokenData,
}

impl SyntaxTokenItem {
    pub fn value<'a>(&'a self) -> &'a str {
        self.data.raw.text()
    }
}

impl SyntaxTokenItem {
    pub(crate) fn from_raw(data: SyntaxTokenData) -> Self {
        Self { data }
    }
}

impl MetadataAccess for SyntaxTokenItem {
    fn metadata_key(&self) -> NodeMetadataKey {
        self.data.metadata_key()
    }

    fn metadata(&self) -> NodeMetadata {
        self.data.metadata()
    }
}

impl NodeOperation for SyntaxTokenItem {
    type Item = SyntaxTokenItem;
    type Parent = SyntaxTokenSet;

    fn parent(&self) -> Option<Self::Parent> {
        self.data.raw.parent()
        .map(|raw| {
            SyntaxTokenSet::from_raw(SyntaxNodeData::new(
                raw, 
                self.data.metadata_table.clone(), 
                self.data.parse_mode.clone(), 
                self.data.engine
            ))
        })
    }

    fn prev_sibling(&self) -> Option<Self::Item> {
        self.data.raw.prev_token()
        .map(|raw| {
            SyntaxTokenItem::from_raw(SyntaxTokenData::new(
                raw, 
                self.data.metadata_table.clone(), 
                self.data.parse_mode.clone(), 
                self.data.engine
            ))
        })
    }

    fn next_sibling(&self) -> Option<Self::Item> {
        self.data.raw.next_token()
        .map(|raw| {
            SyntaxTokenItem::from_raw(SyntaxTokenData::new(
                raw, 
                self.data.metadata_table.clone(), 
                self.data.parse_mode.clone(), 
                self.data.engine
            ))
        })
    }
}

pub struct SyntaxTokenItems {
    children: rowan::SyntaxElementChildren<RowanLangageImpl>,
    metadata_table: Rc<Vec<StatementMetadataMap>>,
    parse_mode: ParseMode,
    engine: ParsingRuleSet,
    node_type: NodeType,
}

impl Iterator for SyntaxTokenItems {
    type Item = SyntaxTokenItem;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(rowan::NodeOrToken::Token(item)) = self.children.next() {
            let data = SyntaxTokenData::new(item, self.metadata_table.clone(), self.parse_mode.clone(), self.engine);
            let metadata = data.metadata();

            if metadata.node_type == self.node_type {
                return Some(SyntaxTokenItem::from_raw(data));
            }
        }

        None
    }
}

impl SyntaxTokenItems {
    pub(crate) fn new(data: &SyntaxNodeData, node_type: NodeType) -> Self {
        Self {
            children: data.raw.children_with_tokens(),
            metadata_table: data.metadata_table.clone(),
            parse_mode: data.parse_mode.clone(),
            engine: data.engine,
            node_type,
        }
    }
}
