#![cfg(not(engine_ungenerated))]

use parser_core::{event_dispatcher::ParseEvent, node_handler::{NodeBuildError, SyntaxTreeBuilder}};
use sqlite_engine::syntax_kind;

mod build_tree_tests {
    use parser_core::{syntax_tree::{MetadataAccess, SyntaxElement, SyntaxNode, SyntaxTokenItem, SyntaxTokenSet, SyntaxTriviaItems}, NodeMetadata, NodeMetadataKey, NodeType};

    use super::*;

    #[derive(Debug)]
    enum ActualNode {
        Node(SyntaxNode),
        TokenSet(SyntaxTokenSet),
        TokenItem(SyntaxTokenItem),
    }

    #[derive(Debug)]
    enum ExpectNode<'a> {
        Node { metadata: ExpectMetadata, children: &'a [ExpectNode<'a>] },
        TokenSet { metadata: ExpectMetadata, leading: &'a [ExpectNode<'a>], token: Box<ExpectNode<'a>>, trailing: &'a [ExpectNode<'a>] },
        TokenItem { metadata: ExpectMetadata, value: &'a str },
    }


    #[derive(Debug)]
    struct ExpectMetadata(NodeMetadataKey, NodeMetadata);

    fn verify<'a>(actual: &ActualNode, expect_node: &ExpectNode<'a>, depth: usize) {
        match (actual, expect_node) {
            (ActualNode::Node(actual), ExpectNode::Node { metadata, children }) => {
                verify_member(actual, metadata);
                assert_eq!(children.len(), actual.children().count());

                for (child, expect_child) in actual.children().zip(*children) {
                    match &child {
                        SyntaxElement::Node(child_node) => {
                            verify(&ActualNode::Node(child_node.clone()), expect_child, depth+1);
                        }
                        SyntaxElement::TokenSet(token_set) => {
                            verify(&ActualNode::TokenSet(token_set.clone()), expect_child, depth+1);
                        }
                    }
                }
            }
            (ActualNode::TokenSet(actual), ExpectNode::TokenSet { metadata, leading, token, trailing }) => {
                verify_member(actual, metadata);
                verify(&ActualNode::TokenItem(actual.token()), token, depth+1);
                verify_trivia(actual.leading_trivia(), actual.leading_trivia().count(), leading, depth+1);
                verify_trivia(actual.trailing_trivia(), actual.trailing_trivia().count(), trailing, depth+1);
            }
            (ActualNode::TokenItem(actual), ExpectNode::TokenItem { metadata, value }) => {
                verify_member(actual, metadata);
                assert_eq!(*value, actual.value());
            }
            (lhs, rhs) => {
                panic!("Unexpected convination (lhs: {lhs:?}, rhs: {rhs:?})");
            }
        }
    }

    fn verify_member<'a>(member: &'a impl MetadataAccess, ExpectMetadata(key, metadata): &ExpectMetadata) {
        'verify_key: {
            assert_eq!(*key, member.metadata_key());
            break 'verify_key;
        }
        'verify_metadata: {
            assert_eq!(metadata, member.metadata());
            break 'verify_metadata;
        }
    }

    fn verify_trivia(trivia_items: SyntaxTriviaItems, actual_count: usize, expect: &[ExpectNode], depth: usize) {
        assert_eq!(expect.len(), actual_count);

        for (item, expect_item) in trivia_items.zip(expect) {
            verify(&ActualNode::TokenItem(item), expect_item, depth+1);
        }
    }

    #[test]
    fn test_create_empty_syntax() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let handler = SyntaxTreeBuilder::new(engine.parsing_rules);
        let event = ParseEvent::Accept { kind: syntax_kind::r#input, last_state: 0, edit_state: 0 };
        assert_eq!(Err(NodeBuildError::EmptyTree), handler.build(event));
        Ok(())
    }

    #[test]
    fn test_create_eof_only() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let mut handler = SyntaxTreeBuilder::new(engine.parsing_rules);
        let event = ParseEvent::Shift { kind: syntax_kind::r#EOF, current_state: 0, next_state: 0, edit_state: 0 };
        assert_eq!(Ok(()), handler.add_kind_token(event));
        
        let event = ParseEvent::Accept { kind: syntax_kind::r#input, last_state: 0, edit_state: 0 };
        let tree = handler.build(event);
        assert!(tree.is_ok());

        let tree = tree.unwrap();

        let expect_tree = ExpectNode::Node{
            metadata: ExpectMetadata(
                NodeMetadataKey{ kind: syntax_kind::input, offset: 0, len: 0, is_leaf: false },
                NodeMetadata{ edit_state: 0, node_type: NodeType::Node, recovery: None, char_offset: 0, char_len: 0 }
            ),
            children: &[
                ExpectNode::TokenSet { 
                    metadata: ExpectMetadata(
                        NodeMetadataKey { kind: syntax_kind::r#EOF, offset: 0, len: 0, is_leaf: false },
                        NodeMetadata { edit_state: 0, node_type: NodeType::TokenSet, recovery: None, char_offset: 0, char_len: 0 }
                    ),
                    leading: &[],
                    token: Box::new(ExpectNode::TokenItem { 
                        metadata: ExpectMetadata(
                            NodeMetadataKey { kind: syntax_kind::r#EOF, offset: 0, len: 0, is_leaf: true },
                            NodeMetadata { edit_state: 0, node_type: NodeType::TokenItem, recovery: None, char_offset: 0, char_len: 0 }
                        ),
                        value: "",
                    }),
                    trailing: &[],
                }
            ]
        };
        verify(&&ActualNode::Node(tree.root()), &expect_tree, 0);
        // 'root_node: {
        //     let root = tree.root();
        //     // assert_eq!(syntax_kind::r#input, root.kind());
        //     // assert_eq!(1, root.children_with_tokenset().count());
        //     // assert_eq!(0, root.start_as_byte());
        //     // assert_eq!(0, root.len_as_byte());

        //     // let metadata = root.metadata();
        //     // assert_eq!(NodeType::Node, metadata.node_type());

        //     let cursor = root.cursor();
        //     let child = cursor.nth_child(0);
        //     assert_eq!(true, child.is_some());
            
        //     let child = child.unwrap();

        //     let key = child.metadata_key();
        //     assert_eq!(syntax_kind::r#EOF, key.kind);
        //     assert_eq!(0, key.offset);
        //     assert_eq!(0, key.len);

        //     let metadata = child.metadata();
        //     assert_eq!(NodeType::TokenSet, metadata.node_type);
        //     assert_eq!(0, metadata.edit_state);
        //     assert_eq!(None, metadata.recovery);
        //     assert_eq!(0, metadata.char_offset);
        //     assert_eq!(0, metadata.char_len);

        //     let child = child.to_token_set().unwrap();
            
        //     let trivias = child.leading_trivia();
        //     assert_eq!(0, trivias.count());
        //     let trivias = child.trailing_trivia();
        //     assert_eq!(0, trivias.count());
            
        //     let token = child.token();
        //     assert_eq!("", token.value());

        //     let key = token.metadata_key();
        //     assert_eq!(syntax_kind::r#EOF, key.kind);
        //     assert_eq!(0, key.offset);
        //     assert_eq!(0, key.len);
        //     break 'root_node;
        // }

        Ok(())
    }

    #[test]
    fn test_create_token_for_main_only_lookahead() -> Result<(), anyhow::Error> {
        todo!()
    }

    #[test]
    fn test_create_token_with_leading_trivia() -> Result<(), anyhow::Error> {
        todo!()
    }

    #[test]
    fn test_create_token_with_trailing_trivia() -> Result<(), anyhow::Error> {
        todo!()
    }

    #[test]
    fn test_create_token_with_full_set() -> Result<(), anyhow::Error> {
        todo!()
    }

    #[test]
    fn test_create_node() -> Result<(), anyhow::Error> {
        todo!()
    }

    #[test]
    fn test_resolve_node_syntax_kind() -> Result<(), anyhow::Error> {
        todo!()
    }
}