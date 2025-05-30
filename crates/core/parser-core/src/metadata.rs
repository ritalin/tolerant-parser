use std::collections::HashMap;

use engine_core::SyntaxKind;

use crate::NodeId;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct NodeMetadataKey {
    pub kind: SyntaxKind,
    pub offset: usize,
    pub len: usize,
    pub is_leaf: bool,
}

impl NodeMetadataKey {
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
        Self {
            edit_state: self.edit_state,
            node_type: self.node_type,
            patch: self.patch,
            char_offset: self.char_offset - stmt_offset,
            char_len: self.char_len,
        }
    }

    pub fn into_global(&self, stmt_offset: usize) -> Self {
        Self {
            edit_state: self.edit_state,
            node_type: self.node_type.clone(),
            patch: self.patch.clone(),
            char_offset: self.char_offset + stmt_offset,
            char_len: self.char_len,
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

#[derive(PartialEq, Clone, Debug)]
pub struct StatementMetadataMap {
    pub byte_offset: usize,
    pub char_offset: usize,
    pub map: HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>
}