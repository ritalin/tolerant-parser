#![cfg(not(engine_ungenerated))]

use parser_core::{event_dispatcher::ParseEvent, node_handler::{NodeBuildError, SyntaxTreeBuilder}};
use sqlite_engine::syntax_kind;

mod build_tree_tests {
    use std::f32::consts::E;

    use parser_core::{node_handler::{NodeMetadata, NodeMetadataKey}, syntax_tree::{MetadataAccess, NodeType, SyntaxNode}};

    use super::*;

    struct ExpectNode<'a> {
        key: NodeMetadataKey,
        metadata: NodeMetadata,
        children: &'a [ExpectNode<'a>]
    }

    fn verify<'a>(actual: &SyntaxNode, expect_node: &ExpectNode<'a>, depth: usize) {
        'verify_member: {
            verify_member(actual, expect_node);
            break 'verify_member;
        }
        'verify_children: {
            assert_eq!(expect_node.children.len(), actual.children().count());

            for (child, expect_child) in actual.children().zip(expect_node.children) {
                match &child {
                    parser_core::syntax_tree::SyntaxElement::Node(child_node) => {
                        verify(child_node, expect_child, depth+1);
                    }
                    parser_core::syntax_tree::SyntaxElement::TokenSet(token_set) => {
                        verify_member(token_set, expect_child);
                    }
                    parser_core::syntax_tree::SyntaxElement::TokenItem(item) => {
                        verify_member(item, expect_child);
                    }
                }
            }
            break 'verify_children;
        }
    }

    fn verify_member<'a>(member: &'a impl MetadataAccess, expect_node: &'a ExpectNode<'a>) {
        'verify_key: {
            assert_eq!(expect_node.key, member.metadata_key());
            break 'verify_key;
        }
        'verify_metadata: {
            assert_eq!(&expect_node.metadata, member.metadata());
            break 'verify_metadata;
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

        let expect_tree = ExpectNode{
            key: NodeMetadataKey{ kind: syntax_kind::input, offset: 0, len: 0, is_leaf: false },
            metadata: NodeMetadata{ edit_state: 0, node_type: NodeType::Node, recovery: None, char_offset: 0, char_len: 0 },
            children: &[
                ExpectNode{
                    key: NodeMetadataKey { kind: syntax_kind::r#EOF, offset: 0, len: 0, is_leaf: false },
                    metadata: NodeMetadata { edit_state: 0, node_type: NodeType::TokenSet, recovery: None, char_offset: 0, char_len: 0 },
                    children: &[
                        ExpectNode{
                            key: NodeMetadataKey { kind: syntax_kind::r#EOF, offset: 0, len: 0, is_leaf: false },
                            metadata: NodeMetadata { edit_state: 0, node_type: NodeType::TokenItem, recovery: None, char_offset: 0, char_len: 0 },
                            children: &[]
                        }
                    ]
                }
            ]
        };
        verify(&tree.root(), &expect_tree, 0);
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