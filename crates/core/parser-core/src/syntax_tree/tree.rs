use std::rc::Rc;
use engine_core::parser_engine::ParsingRuleSet;
use crate::{metadata::StatementMetadataMap, ParseMode};
use super::{RowanLangageImpl, SyntaxNode};


#[derive(PartialEq, Clone, Debug)]
pub struct SyntaxTree {
    root: rowan::SyntaxNode<RowanLangageImpl>,
    metadata_table: Rc<Vec<StatementMetadataMap>>,
    parse_mode: ParseMode,
    engine: ParsingRuleSet,
}

impl SyntaxTree {
    pub fn root(&self) -> SyntaxNode {
        SyntaxNode::new(self.root.clone(), self.metadata_table.clone(), self.parse_mode.clone(), self.engine)
    }
}

impl SyntaxTree {
    pub (crate) fn new(
        root: rowan::GreenNode, 
        metadata_table: Vec<StatementMetadataMap>,
        parse_mode: ParseMode,
        engine: ParsingRuleSet) -> Self 
    {
        Self {
            root: rowan::api::SyntaxNode::new_root_mut(root),
            metadata_table: Rc::new(metadata_table),
            parse_mode,
            engine,
        }
    }
}

impl SyntaxTree {
    pub(crate) fn metadata_table(&self) -> Rc<Vec<StatementMetadataMap>> {
        self.metadata_table.clone()
    }
}