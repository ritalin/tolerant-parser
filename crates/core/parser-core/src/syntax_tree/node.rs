use std::{collections::HashMap, rc::Rc};
use engine_core::parser_engine::ParsingRuleSet;
use crate::{NodeId, NodeMetadata, NodeMetadataKey, NodeType};
use super::{MetadataAccess, NodeOperation, RowanLangageImpl, SyntaxElement, SyntaxNodeData, SyntaxTokenSet};

#[derive(Clone, Debug)]
pub struct SyntaxNode {
    data: SyntaxNodeData,
}

impl SyntaxNode {
    pub fn new(
        raw: rowan::SyntaxNode<RowanLangageImpl>, 
        metadata_map: Rc<HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>>,
        engine: ParsingRuleSet) -> Self {
        Self { data: SyntaxNodeData::new(raw, metadata_map, engine) }
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
                let data = self.data.with_raw(node);

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
}

impl MetadataAccess for SyntaxNode {
    fn metadata_key(&self) -> NodeMetadataKey {
        self.data.metadata_key()
    }

    fn metadata(&self) -> &NodeMetadata {
        self.data.metadata()
    }
}

impl SyntaxNode {
    pub(crate) fn from_raw(data: SyntaxNodeData) -> Self {
        Self { data }
    }
}

impl NodeOperation for SyntaxNode {
    type Item = SyntaxNode;

    fn parent(&self) -> Option<SyntaxNode> {
        match self.data.raw.parent() {
            Some(node) => {
                Some(SyntaxNode::from_raw(self.data.with_raw(&node)))
            }
            None => None
        }
    }

    fn prev_sibling(&self) -> Option<Self::Item> {
        todo!()
    }

    fn next_sibling(&self) -> Option<Self::Item> {
        todo!()
    }
}

pub struct SyntaxNodeChildren {
    raw: rowan::SyntaxNodeChildren<RowanLangageImpl>,
    metadata_map: Rc<HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>>,
    engine: ParsingRuleSet
}

impl SyntaxNodeChildren {
    pub(crate) fn new(data: SyntaxNodeData) -> Self {
        Self { 
            raw: data.raw.children(),
            metadata_map: data.metadata_map.clone(),
            engine: data.engine
        }
    }
}

impl Iterator for SyntaxNodeChildren {
    type Item = SyntaxElement;

    fn next(&mut self) -> Option<Self::Item> {
        match self.raw.next() {
            Some(node) => {
                let data = SyntaxNodeData::new(node, self.metadata_map.clone(), self.engine);

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
