use std::rc::Rc;
use engine_core::parser_engine::ParsingRuleSet;
use crate::{metadata::StatementMetadataMap, NodeMetadata, NodeMetadataKey, NodeType, ParseMode};
use super::{MetadataAccess, NodeOperation, RowanLangageImpl, SyntaxNodeData, SyntaxTokenData};

#[derive(Clone, Debug)]
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

    fn metadata(&self) -> &NodeMetadata {
        self.data.metadata()
    }
}

impl NodeOperation for SyntaxTokenSet {
    type Item = SyntaxTokenSet;

    fn parent(&self) -> Option<super::SyntaxNode> {
        todo!()
    }

    fn prev_sibling(&self) -> Option<Self::Item> {
        todo!()
    }

    fn next_sibling(&self) -> Option<Self::Item> {
        todo!()
    }
}

impl SyntaxTokenSet {
    pub(crate) fn from_raw(data: SyntaxNodeData) -> Self {
        Self { data }
    }
}

#[derive(Clone, Debug)]
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

    fn metadata(&self) -> &NodeMetadata {
        self.data.metadata()
    }
}

impl NodeOperation for SyntaxTokenItem {
    type Item = SyntaxTokenItem;

    fn parent(&self) -> Option<super::SyntaxNode> {
        todo!()
    }

    fn prev_sibling(&self) -> Option<Self::Item> {
        todo!()
    }

    fn next_sibling(&self) -> Option<Self::Item> {
        todo!()
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
