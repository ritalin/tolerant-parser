use std::{collections::HashMap, rc::Rc};
use engine_core::parser_engine::ParsingRuleSet;
use crate::{metadata::{MetadataTable, StatementMetadataEntry}, syntax_tree::SyntaxNodeData, NodeMetadata, NodeMetadataKey, ParseMode};
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

pub struct SyntaxFragment {
    index: usize,
    node: rowan::NodeOrToken<rowan::GreenNode, rowan::GreenToken>,
    metadata_entry: StatementMetadataEntry,
    adjusted_byte_offset: usize,
}

impl SyntaxFragment {
    pub fn new(index: usize, node: rowan::NodeOrToken<rowan::GreenNode, rowan::GreenToken>, metadata_map: HashMap<NodeMetadataKey, NodeMetadata>) -> Self {
        Self {
            index,
            node,
            metadata_entry: StatementMetadataEntry{
                map: metadata_map,
                ..Default::default()
            },
            adjusted_byte_offset: 0,
        }
    }

    pub fn adjust_byte_offset(self, byte_offset: usize) -> Self {
        Self {
            adjusted_byte_offset: byte_offset,
            ..self
        }
    }

    pub fn into_root(self, parse_mode: ParseMode, engine: ParsingRuleSet) -> SyntaxNode {
        let red_node = rowan::api::SyntaxNode::<RowanLangageImpl>::new_root(self.node.clone().into_node().unwrap());

        let metadata_table = MetadataTable::new(vec![self.metadata_entry], StatementMetadataEntry::default());

        SyntaxNode::from_raw(SyntaxNodeData::new(red_node, std::rc::Rc::new(metadata_table), parse_mode, engine))
    }

    #[inline]
    pub fn seq(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn byte_offset(&self) -> usize {
        self.adjusted_byte_offset
    }
}

impl std::fmt::Display for SyntaxFragment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let red_node = rowan::api::SyntaxNode::<RowanLangageImpl>::new_root(self.node.clone().into_node().unwrap());

        let mut next_token = red_node.first_token();
        while let Some(token) = next_token {
            write!(f, "{}", token.text())?;
            next_token = token.next_token();
        }

        Ok(())
    }
}

pub struct SyntaxFragmentBatch {
   pub  members: Vec<SyntaxFragment>,
   pub engine: ParsingRuleSet,
   pub parse_mode: ParseMode,
}

impl From<SyntaxFragmentBatch> for SyntaxTree {
    fn from(value: SyntaxFragmentBatch) -> Self {
        let mut members = value.members;
        members.sort_by(|lhs, rhs| lhs.seq().cmp(&rhs.seq()));

        let mut global_byte_offset = 0;
        let mut global_char_offset = 0;

        let (children, metadata_entriess): (Vec<rowan::NodeOrToken<rowan::GreenNode, rowan::GreenToken>>, Vec<StatementMetadataEntry>) = 
            members.into_iter()
            .map(|member| {
                let metadata_entry = StatementMetadataEntry{ 
                    byte_offset: global_byte_offset, 
                    char_offset: global_char_offset, 
                    map: member.metadata_entry.map 
                };
                global_byte_offset += usize::from(member.node.text_len());
                global_char_offset += measure_statement_char_len(std::borrow::Borrow::borrow(member.node.as_node().unwrap()));

                (member.node, metadata_entry)
            })
            .unzip()
        ;

        let root_key = NodeMetadataKey{ 
            kind: value.engine.full_emit_config().from_symbol, 
            offset: 0, len: global_byte_offset, is_leaf: false 
        };
        let root_metadata = NodeMetadata{ 
            edit_state: 0, 
            node_type: crate::NodeType::Node, 
            patch: crate::PatchAction::None, 
            char_offset: 0, 
            char_len: global_char_offset 
        };
        let root_metadata_entry = StatementMetadataEntry{ 
            map: HashMap::from([(root_key, root_metadata)]), 
            ..Default::default() 
        };
        let root_kind_id = rowan::SyntaxKind(value.engine.full_emit_config().from_symbol.id as u16);
        let root_node = rowan::GreenNode::new(root_kind_id, children);

        let metadata_table = MetadataTable::new(metadata_entriess, root_metadata_entry);

        SyntaxTree::new(root_node, metadata_table, value.parse_mode, value.engine)
    }
}

fn measure_statement_char_len(root: &rowan::GreenNodeData) -> usize {
    let mut stack = vec![rowan::NodeOrToken::Node(root)];
    let mut size = 0;

    while let Some(node) = stack.pop() {
        match node {
            rowan::NodeOrToken::Node(node) => {
                stack.extend(node.children());
            }
            rowan::NodeOrToken::Token(token) => {
                size += token.text().chars().count();
            }
        }
    }

    size
}
