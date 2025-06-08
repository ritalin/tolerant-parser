use std::collections::HashMap;
use engine_core::{parser_engine::ParsingRuleSet, SyntaxKind};
use crate::syntax_tree::RowanLangageImpl;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct NodeMetadataKey {
    pub kind: SyntaxKind,
    pub offset: usize,
    pub len: usize,
    pub is_leaf: bool,
}

impl NodeMetadataKey {
    pub fn from_raw_node(node: &rowan::SyntaxNode<RowanLangageImpl>, engine: ParsingRuleSet) -> Self {
        let range = node.text_range();
        Self {
            kind: engine.from_kind_id(node.kind()),
            offset: range.start().into(),
            len: range.len().into(),
            is_leaf: false,
        }
    }

    pub fn from_raw_token(node: &rowan::SyntaxToken<RowanLangageImpl>, engine: ParsingRuleSet) -> Self {
        let range = node.text_range();
        Self {
            kind: engine.from_kind_id(node.kind()),
            offset: range.start().into(),
            len: range.len().into(),
            is_leaf: true,
        }
    }

    pub fn into_local(self, stmt_offset: usize) -> Self {
        Self {
            kind: self.kind,
            offset: self.offset - stmt_offset,
            len: self.len,
            is_leaf: self.is_leaf,
        }
    }

    pub fn into_global(self, stmt_offset: usize) -> Self {
        Self {
            kind: self.kind,
            offset: self.offset + stmt_offset,
            len: self.len,
            is_leaf: self.is_leaf,
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct NodeMetadata {
    pub edit_state: usize,
    pub node_type: NodeType,
    pub patch: PatchAction,
    pub char_offset: usize,
    pub char_len: usize,
}

impl NodeMetadata {
    pub fn into_local(self, stmt_offset: usize) -> Self {
        match stmt_offset {
            0 => self,
            _ => {
                Self {
                    edit_state: self.edit_state,
                    node_type: self.node_type,
                    patch: self.patch,
                    char_offset: self.char_offset - stmt_offset,
                    char_len: self.char_len,
                }
            }
        }
    }

    pub fn into_global(&self, stmt_offset: usize) -> Self {
        match stmt_offset {
            0 => self.clone(),
            _ => {
                Self {
                    edit_state: self.edit_state,
                    node_type: self.node_type.clone(),
                    patch: self.patch.clone(),
                    char_offset: self.char_offset + stmt_offset,
                    char_len: self.char_len,
                }
            }
        }
    }
}

#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum PatchAction {
    None,
    Delete,
    Shift,
    Invalid,
}

impl std::fmt::Display for PatchAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PatchAction::None => "None",
            PatchAction::Delete => "Delete",
            PatchAction::Shift => "Shift",
            PatchAction::Invalid => "Invalid",
        };
        write!(f, "{}", s)
    }
}

#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum NodeType {
    Node,
    TokenSet,
    TokenItem,
    LeadingToken,
    TrailingToken,
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            NodeType::Node => "Node",
            NodeType::TokenSet => "TokenSet",
            NodeType::TokenItem => "TokenItem",
            NodeType::LeadingToken => "LeadingToken",
            NodeType::TrailingToken => "TrailingToken",
        };

        write!(f, "{name}")
    }
}

#[derive(PartialEq, Clone, Default, Debug)]
pub struct StatementMetadataEntry {
    pub byte_offset: usize,
    pub char_offset: usize,
    pub map: HashMap<NodeMetadataKey, NodeMetadata>
}

#[derive(PartialEq, Clone, Debug)]
pub struct MetadataTable {
    members: Vec<StatementMetadataEntry>,
    root: StatementMetadataEntry,
}

impl MetadataTable {
    pub fn new(members: Vec<StatementMetadataEntry>, root: StatementMetadataEntry) -> Self {
        Self { members, root }
    }

    pub fn statement_metadata(&self, index: Option<usize>) -> &StatementMetadataEntry {
        if let Some(index) = index {
            if let Some(member) = self.members.get(index) {
                return member;
            }
        }

        &self.root
    }

    /// Get last entry index less than or equal specified offset.
    /// If not found, return 0.
    pub fn index_at_offset(&self, byte_offset: usize) -> usize {
        self.members.iter().enumerate()
        .take_while(|(_, member)| member.byte_offset <= byte_offset)
        .map(|(i, _)| i)
        .last()
        .unwrap_or_default()
    }

    pub fn clone_members(&self) -> Vec<StatementMetadataEntry> {
        self.members.clone()
    }
}