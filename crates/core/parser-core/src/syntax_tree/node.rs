use std::rc::Rc;
use engine_core::parser_engine::ParsingRuleSet;
use crate::{metadata::MetadataTable, NodeMetadata, NodeMetadataKey, NodeType, ParseMode};
use super::{MetadataAccess, NodeOperation, RowanLangageImpl, SyntaxElement, SyntaxNodeData, SyntaxTokenData, SyntaxTokenItem, SyntaxTokenSet};

#[derive(PartialEq, Clone, Debug)]
pub struct SyntaxNode {
    data: SyntaxNodeData,
}

impl SyntaxNode {
    pub fn new(
        raw: rowan::SyntaxNode<RowanLangageImpl>, 
        metadata_table: Rc<MetadataTable>,
        parse_mode: ParseMode,
        engine: ParsingRuleSet) -> Self {
        Self { data: SyntaxNodeData::new(raw, metadata_table, parse_mode, engine) }
    }

    pub fn into_raw(&self) -> rowan::SyntaxNode<RowanLangageImpl> {
        self.data.raw.clone()
    }
}

impl SyntaxNode {
    pub fn nth_child(&self, index: usize) -> Option<SyntaxElement> {
        let child = match index {
            0 => self.data.raw.first_child(),
            _ => self.data.raw.children().nth(index)
        };

        match child.as_ref() {
            Some(node) => {
                let data = self.data.with_raw(node, self.data.parse_mode.clone());

                match data.metadata().node_type {
                    NodeType::Node => {
                        Some(SyntaxElement::Node(SyntaxNode::from_raw(data)))
                    }
                    NodeType::TokenSet => {
                        Some(SyntaxElement::TokenSet(SyntaxTokenSet::from_raw(data)))
                    }
                    _ => None
                }
            }
            None => None,
        }
    }

    pub fn children(&self) -> SyntaxNodeChildren {
        SyntaxNodeChildren::new(self.data.clone())
    }

    pub fn token_at_offset(&self, offset: usize) -> Option<SyntaxTokenItem> {
        let token = match self.data.raw.token_at_offset((offset as u32).into()) {
            rowan::TokenAtOffset::None => None,
            rowan::TokenAtOffset::Single(token) => Some(token),
            rowan::TokenAtOffset::Between(_, token) => Some(token),
        };

        token.map(|raw| {
            SyntaxTokenItem::from_raw(SyntaxTokenData::new(
                raw, 
                self.data.metadata_table.clone(), 
                self.data.parse_mode.clone(), 
                self.data.engine
            ))
        })
    }
}

impl MetadataAccess for SyntaxNode {
    fn metadata_key(&self) -> NodeMetadataKey {
        self.data.metadata_key()
    }

    fn metadata(&self) -> NodeMetadata {
        self.data.metadata()
    }
}

impl SyntaxNode {
    pub(crate) fn from_raw(data: SyntaxNodeData) -> Self {
        Self { data }
    }
}

impl NodeOperation for SyntaxNode {
    type Item = SyntaxElement;
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
        }
    }
}

pub struct SyntaxNodeChildren {
    raw: rowan::SyntaxNodeChildren<RowanLangageImpl>,
    metadata_table: Rc<MetadataTable>,
    parse_mode: ParseMode,
    engine: ParsingRuleSet
}

impl SyntaxNodeChildren {
    pub(crate) fn new(data: SyntaxNodeData) -> Self {
        Self { 
            raw: data.raw.children(),
            metadata_table: data.metadata_table.clone(),
            parse_mode: data.parse_mode,
            engine: data.engine
        }
    }
}

impl Iterator for SyntaxNodeChildren {
    type Item = SyntaxElement;

    fn next(&mut self) -> Option<Self::Item> {
        match self.raw.next() {
            Some(node) => {
                let data = SyntaxNodeData::new(node, self.metadata_table.clone(), self.parse_mode.clone(), self.engine);

                match data.metadata().node_type {
                    NodeType::Node => {
                        Some(SyntaxElement::Node(SyntaxNode::from_raw(data)))
                    }
                    NodeType::TokenSet => {
                        Some(SyntaxElement::TokenSet(SyntaxTokenSet::from_raw(data)))
                    }
                    _ => None
                }
            }
            None => None
        }
    }
}
