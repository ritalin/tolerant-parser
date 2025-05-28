use engine_core::parser_engine::ParsingRuleSet;
use parser_core::{syntax_tree::{MetadataAccess, SyntaxElement, SyntaxNode, SyntaxTokenItem, SyntaxTokenItems, SyntaxTokenSet}, NodeMetadata, NodeMetadataKey, PatchAction};
use serde::ser::SerializeStruct;

#[derive(Debug)]
pub enum ActualNode {
    Node(SyntaxNode),
    TokenSet(SyntaxTokenSet),
    TokenItem(SyntaxTokenItem),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub enum ExpectNode {
    Node { metadata: ExpectMetadata, children: Vec<ExpectNode> },
    TokenSet { metadata: ExpectMetadata, leading: Vec<ExpectNode>, token: Box<ExpectNode>, trailing: Vec<ExpectNode> },
    TokenItem { metadata: ExpectMetadata, value: String },
}

#[derive(Debug)]
pub struct ExpectMetadata(pub ExpectMetadataKey, pub ExpectMetadataValue);

pub fn verify<'a>(actual: &ActualNode, expect_node: &ExpectNode, engine: ParsingRuleSet, depth: usize) {
    match (actual, expect_node) {
        (ActualNode::Node(actual), ExpectNode::Node { metadata, children }) => {
            verify_member(actual, metadata, engine, depth);
            assert_eq!(expect_kinds(children, engine), actual_kinds(actual.children()));

            for (child, expect_child) in actual.children().zip(children) {
                match &child {
                    SyntaxElement::Node(child_node) => {
                        verify(&ActualNode::Node(child_node.clone()), expect_child, engine, depth+1);
                    }
                    SyntaxElement::TokenSet(token_set) => {
                        verify(&ActualNode::TokenSet(token_set.clone()), expect_child, engine, depth+1);
                    }
                }
            }
        }
        (ActualNode::TokenSet(actual), ExpectNode::TokenSet { metadata, leading, token, trailing }) => {
            verify_member(actual, metadata, engine, depth);
            verify(&ActualNode::TokenItem(actual.token()), token, engine, depth+1);
            verify_trivia(actual.leading_trivia(), actual.leading_trivia().count(), leading, engine, depth+1);
            verify_trivia(actual.trailing_trivia(), actual.trailing_trivia().count(), trailing, engine, depth+1);
        }
        (ActualNode::TokenItem(actual), ExpectNode::TokenItem { metadata, value }) => {
            verify_member(actual, metadata, engine, depth);
            assert_eq!(format!("{:?}", value), format!("{:?}", actual.value()));
        }
        (lhs, rhs) => {
            panic!("Unexpected convination (lhs: {lhs:?}, rhs: {rhs:?})");
        }
    }
}

fn verify_member<'a>(member: &'a impl MetadataAccess, ExpectMetadata(key, metadata): &ExpectMetadata, engine: ParsingRuleSet, _actual_depth: usize) {
    assert_eq!((key.of(engine), &metadata.of()), (member.metadata_key(), member.metadata()));
}

fn verify_trivia(trivia_items: SyntaxTokenItems, actual_count: usize, expect: &[ExpectNode], engine: ParsingRuleSet, depth: usize) {
    assert_eq!(expect.len(), actual_count);

    for (item, expect_item) in trivia_items.map(std::convert::identity).zip(expect) {
        verify(&ActualNode::TokenItem(item), expect_item, engine, depth+1);
    }
}

fn expect_kinds<'a>(children: &'a [ExpectNode], engine: ParsingRuleSet) -> Vec<String> {
    children.iter()
    .map(|child| match child {
        ExpectNode::Node { metadata: ExpectMetadata(key, _), .. } => key.of(engine).kind.text.to_string(),
        ExpectNode::TokenSet { metadata: ExpectMetadata(key, _), .. } => key.of(engine).kind.text.to_string(),
        ExpectNode::TokenItem { metadata: ExpectMetadata(key, _), .. } => key.of(engine).kind.text.to_string(),
    })
    .collect()
}

fn actual_kinds(iter: impl Iterator<Item = SyntaxElement>) -> Vec<&'static str> {
    iter
    .map(|child| child.metadata_key().kind.text)
    .collect()
}

impl serde::Serialize for ExpectMetadata {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer 
    {
        let ExpectMetadata(key, metadata) = self;
        
        let mut ser = serializer.serialize_struct(std::any::type_name::<ExpectMetadata>(), 9)?;
        ser.serialize_field("key", &key)?;
        ser.serialize_field("metadata", &metadata)?;
        ser.end()
    }
}

impl<'de> serde::Deserialize<'de> for ExpectMetadata {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: serde::Deserializer<'de> 
    {
        #[derive(serde::Deserialize)]
        struct DeserValue { key: ExpectMetadataKey, value: ExpectMetadataValue }

        let DeserValue { key, value } = DeserValue::deserialize(deserializer)?;
        Ok(ExpectMetadata(key, value))
    }
}

#[derive(PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct ExpectMetadataKey {
    pub kind_id: u32,
    pub kind_name: String,
    pub byte_offset: usize,
    pub byte_len: usize,
    pub is_leaf: bool,
}

impl ExpectMetadataKey {
    pub fn of(&self, engine: ParsingRuleSet) -> NodeMetadataKey {
        NodeMetadataKey{ kind: engine.from_kind_id(self.kind_id), offset: self.byte_offset, len: self.byte_len, is_leaf: self.is_leaf }
    }
}

impl From<&NodeMetadataKey> for ExpectMetadataKey {
    fn from(value: &NodeMetadataKey) -> Self {
        Self{ kind_id: value.kind.id, kind_name: value.kind.text.to_string(), byte_offset: value.offset, byte_len: value.len, is_leaf: value.is_leaf }
    }
}

#[derive(PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct ExpectMetadataValue {
    pub edit_state: usize,
    pub node_type: parser_core::NodeType,
    pub patch: PatchAction,
    pub char_offset: usize,
    pub char_len: usize,
}

impl ExpectMetadataValue {
    pub fn of(&self) -> NodeMetadata {
        NodeMetadata{ edit_state: self.edit_state, node_type: self.node_type.clone(), patch: self.patch.clone(), char_offset: self.char_offset, char_len: self.char_len }
    }
}

impl From<&NodeMetadata> for ExpectMetadataValue {
    fn from(value: &NodeMetadata) -> Self {
        Self { edit_state: value.edit_state, node_type: value.node_type.clone(), patch: value.patch.clone(), char_offset: value.char_offset, char_len: value.char_len }
    }
}