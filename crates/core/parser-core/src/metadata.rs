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

#[derive(PartialEq, Debug)]
pub enum Recovery {
    Delete,
    Shift,
}

#[derive(PartialEq, Debug)]
pub enum NodeType {
    Node,
    TokenSet,
    TokenItem,
    LeadingToken,
    TrailingToken,
    Error,
}
