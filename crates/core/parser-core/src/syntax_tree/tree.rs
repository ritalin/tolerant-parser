use std::{collections::HashMap, rc::Rc};
use engine_core::parser_engine::ParsingRuleSet;
use crate::{NodeId, NodeMetadata, NodeMetadataKey};
use super::{RowanLangageImpl, SyntaxNode};


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