use engine_core::SyntaxKind;

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct NodeMetadataKey {
    pub kind: SyntaxKind,
    pub offset: usize,
    pub len: usize,
    pub is_leaf: bool,
}

#[derive(PartialEq, Debug)]
pub struct NodeMetadata {
    pub edit_state: usize,
    pub node_type: NodeType,
    pub recovery: Option<Recovery>,
    pub char_offset: usize,
    pub char_len: usize,
}

#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Recovery {
    Delete,
    Shift,
}

#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum NodeType {
    Node,
    TokenSet,
    TokenItem,
    LeadingToken,
    TrailingToken,
    Error,
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            NodeType::Node => "Node",
            NodeType::TokenSet => "TokenSet",
            NodeType::TokenItem => "TokenItem",
            NodeType::LeadingToken => "LeadingToken",
            NodeType::TrailingToken => "TrailingToken",
            NodeType::Error => "Error",
        };

        write!(f, "{name}")
    }
}