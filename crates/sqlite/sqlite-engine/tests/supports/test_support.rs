use parser_core::{syntax_tree::{MetadataAccess, SyntaxNode, SyntaxTokenItem, SyntaxTokenSet}, NodeMetadata, NodeMetadataKey, PatchAction};

#[derive(PartialEq, Debug)]
pub enum ActualNode {
    Node(SyntaxNode),
    TokenSet(SyntaxTokenSet),
    TokenItem(SyntaxTokenItem),
}

impl MetadataAccess for ActualNode {
    fn metadata_key(&self) -> NodeMetadataKey {
        match self {
            ActualNode::Node(node) => node.metadata_key(),
            ActualNode::TokenSet(token_set) => token_set.metadata_key(),
            ActualNode::TokenItem(token_item) => token_item.metadata_key(),
        }
    }

    fn metadata(&self) -> NodeMetadata {
        match self {
            ActualNode::Node(node) => node.metadata(),
            ActualNode::TokenSet(token_set) => token_set.metadata(),
            ActualNode::TokenItem(token_item) => token_item.metadata(),
        }
    }
}

#[derive(PartialEq, Debug, serde::Deserialize)]
pub struct ExpectNode {
    pub path: Vec<String>,
    pub meta_key: ExpectMetadataKey,
    pub meta_obj: ExpectMetadataValue,
    pub value: Option<String>,
}

#[derive(PartialEq, Debug, serde::Deserialize)]
pub struct ExpectMetadataKey {
    pub byte_offset: usize,
    pub byte_len: usize,
    pub is_leaf: bool,
}

impl From<NodeMetadataKey> for ExpectMetadataKey {
    fn from(value: NodeMetadataKey) -> Self {
        Self {
            byte_offset: value.offset,
            byte_len: value.len,
            is_leaf: value.is_leaf,
        }
    }
}

#[derive(PartialEq, Debug, serde::Deserialize)]
pub struct ExpectMetadataValue {
    pub char_offset: usize,
    pub char_len: usize,
    pub node_type: parser_core::NodeType,
    pub patch: PatchAction,
    pub edit_state: usize,
}

impl From<NodeMetadata> for ExpectMetadataValue {
    fn from(value: NodeMetadata) -> Self {
        Self {
            char_offset: value.char_offset,
            char_len: value.char_len,
            node_type: value.node_type,
            patch: value.patch,
            edit_state: value.edit_state,
        }
    }
}

pub fn verify(actual_node: SyntaxNode, expect_nodes: &[ExpectNode]) {
    let mut stack = vec![(vec![actual_node.metadata_key().kind.text.to_string()], ActualNode::Node(actual_node))];

    let mut i = 0;

    while let Some((path, node)) = stack.pop() {
        let expect = &expect_nodes[i];
        assert_eq!(expect.path, path);
        assert_eq!(expect.meta_key, ExpectMetadataKey::from(node.metadata_key()), "Unmatch key for {:?}", &path);
        assert_eq!(expect.meta_obj, ExpectMetadataValue::from(node.metadata()), "Unmatch metadata for {:?}", &path);

        match node {
            ActualNode::Node(node) => {
                stack.extend(
                    node.children()
                    .map(|x| match x {
                        parser_core::syntax_tree::SyntaxElementDef::Node(item) => {
                            let mut new_path = path.clone();
                            new_path.push(item.metadata_key().kind.text.to_string());

                            (new_path, ActualNode::Node(item))
                        }
                        parser_core::syntax_tree::SyntaxElementDef::TokenSet(item) => {
                            let mut new_path = path.clone();
                            new_path.push(item.metadata_key().kind.text.to_string());

                            (new_path, (ActualNode::TokenSet(item)))
                        }
                    })
                    .collect::<Vec<_>>().into_iter()
                    .rev()
                );
            }
            ActualNode::TokenSet(token_set) => {
                let mut members = vec![];
                members.extend(
                    token_set.leading_trivia().map(|item| {
                        let mut new_path = path.clone();
                        new_path.push(item.metadata_key().kind.text.to_string());

                        (new_path, ActualNode::TokenItem(item))
                    })
                );
                members.push({
                    let item = token_set.token();
                    let mut new_path = path.clone();
                    new_path.push(item.metadata_key().kind.text.to_string());

                    (new_path, ActualNode::TokenItem(item))
                });
                members.extend(
                    token_set.trailing_trivia().map(|item| {
                        let mut new_path = path.clone();
                        new_path.push(item.metadata_key().kind.text.to_string());

                        (new_path, ActualNode::TokenItem(item))
                    })
                );

                stack.extend(members.into_iter().rev());
            }
            ActualNode::TokenItem(token_item) => {
                let v = token_item.value().to_string();
                assert_eq!(expect.value, (!v.is_empty()).then(|| v), "unmatch item value for {:?}", &path);
            }
        }
        i += 1;
    }
}
