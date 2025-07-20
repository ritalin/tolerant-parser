use std::rc::Rc;
use crate::core::engine_core::parser_engine::ParsingRuleSet;
use crate::core::parser_core::{metadata::MetadataTable, syntax_tree::{syntax_batch::ApplyBatch, SyntaxFragmentBatch}, ParseMode};
use super::{RowanLangageImpl, SyntaxNode};

#[derive(PartialEq, Clone, Debug)]
pub struct SyntaxTree {
    root: rowan::SyntaxNode<RowanLangageImpl>,
    metadata_table: Rc<MetadataTable>,
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
        metadata_table: MetadataTable,
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
    pub(crate) fn metadata_table(&self) -> Rc<MetadataTable> {
        self.metadata_table.clone()
    }
}

impl ApplyBatch for SyntaxTree {
    type Output = SyntaxTree;
    
    fn apply_batch(&self, batch: SyntaxFragmentBatch) -> Self::Output {
        super::syntax_batch::apply_batch(&self.root, &self.metadata_table, self.engine, batch)
    }
}