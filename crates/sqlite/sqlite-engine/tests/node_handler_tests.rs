#![cfg(not(engine_ungenerated))]

use parser_core::{event_dispatcher::ParseEvent, node_handler::{NodeBuildError, SyntaxTreeBuilder}};
use sqlite_engine::syntax_kind;

mod build_tree_tests {
    use engine_core::scanner_engine::ScanEvent;
    use parser_core::{syntax_tree::{MetadataAccess, SyntaxElement, SyntaxNode, SyntaxTokenItem, SyntaxTokenSet, SyntaxTokenItems}, NodeMetadata, NodeMetadataKey, NodeType};
    use scanner_core::Token;

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

    fn verify_trivia(trivia_items: SyntaxTokenItems, actual_count: usize, expect: &[ExpectNode], depth: usize) {
        assert_eq!(expect.len(), actual_count);

        for (item, expect_item) in trivia_items.map(std::convert::identity).zip(expect) {
            verify(&ActualNode::TokenItem(item), expect_item, depth+1);
        }
    }

    #[test]
    fn test_create_empty_syntax() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let handler = SyntaxTreeBuilder::new(engine.parsing_rules, None);
        let event = ParseEvent::Accept { kind: syntax_kind::r#input, last_state: 0, edit_state: 0 };
        assert_eq!(Err(NodeBuildError::EmptyTree), handler.build(event));
        Ok(())
    }

    #[test]
    fn test_create_eof_only() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let mut handler = SyntaxTreeBuilder::new(engine.parsing_rules, None);
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
        Ok(())
    }

    #[test]
    fn test_create_token_for_main_only_lookahead() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let mut handler = SyntaxTreeBuilder::new(engine.parsing_rules, None);

        let events = vec![
            ParseEvent::Shift { kind: syntax_kind::r#INTEGER, current_state: 0, next_state: 18, edit_state: 0 },
            ParseEvent::Reduce{ kind: syntax_kind::r#term, current_state: 18, next_state: 29, edit_state: 0, pop_count: 1 },
            ParseEvent::Accept{ kind: syntax_kind::r#expr, last_state: 29, edit_state: 0 },
        ];
        let lookahead = Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::r#INTEGER, offset: 0, len: 2, value: Some("42".into()) },
            trailing_trivia: None,
        };

        handler.add_token_set(events[0].clone(), Some(&lookahead))?;
        handler.add_node(events[1].clone())?;

        let tree = handler.build(events[2].clone())?;

        let expect_tree = ExpectNode::Node{
            metadata: ExpectMetadata(
                NodeMetadataKey{ kind: syntax_kind::expr, offset: 0, len: 2, is_leaf: false },
                NodeMetadata{ edit_state: 0, node_type: NodeType::Node, recovery: None, char_offset: 0, char_len: 2 }
            ),
            children: &[
                ExpectNode::Node { 
                    metadata: ExpectMetadata(
                        NodeMetadataKey{ kind: syntax_kind::term, offset: 0, len: 2, is_leaf: false },
                        NodeMetadata{ edit_state: 0, node_type: NodeType::Node, recovery: None, char_offset: 0, char_len: 2 }
                    ), 
                    children: &[
                        ExpectNode::TokenSet { 
                            metadata: ExpectMetadata(
                                NodeMetadataKey{ kind: syntax_kind::INTEGER, offset: 0, len: 2, is_leaf: false },
                                NodeMetadata{ edit_state: 0, node_type: NodeType::TokenSet, recovery: None, char_offset: 0, char_len: 2 }
                            ), 
                            leading: &[], 
                            token: Box::new(ExpectNode::TokenItem { 
                                metadata: ExpectMetadata(
                                    NodeMetadataKey{ kind: syntax_kind::INTEGER, offset: 0, len: 2, is_leaf: true },
                                    NodeMetadata{ edit_state: 0, node_type: NodeType::TokenItem, recovery: None, char_offset: 0, char_len: 2 }
                                ),
                                value: "42"
                            }), 
                            trailing: &[]
                        }
                    ] 
                }
            ]
        };

        verify(&&ActualNode::Node(tree.root()), &expect_tree, 0);
        Ok(())
    }

    #[test]
    fn test_create_token_with_leading_trivia() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let mut handler = SyntaxTreeBuilder::new(engine.parsing_rules, None);

        let events = vec![
            ParseEvent::Shift { kind: syntax_kind::r#INTEGER, current_state: 0, next_state: 18, edit_state: 0 },
            ParseEvent::Reduce{ kind: syntax_kind::r#term, current_state: 18, next_state: 29, edit_state: 0, pop_count: 1 },
            ParseEvent::Accept{ kind: syntax_kind::r#expr, last_state: 29, edit_state: 0 },
        ];
        let lookahead = Token{
            leading_trivia: Some(vec![
                ScanEvent{ kind: syntax_kind::r#COMMENT, offset: 0, len: 21, value: Some("/* 123あいうabc */".into()) },
                ScanEvent{ kind: syntax_kind::r#SPACE, offset: 21, len: 1, value: Some(" ".into()) },
            ]),
            main: ScanEvent{ kind: syntax_kind::r#INTEGER, offset: 22, len: 2, value: Some("42".into()) },
            trailing_trivia: None,
        };

        handler.add_token_set(events[0].clone(), Some(&lookahead))?;
        handler.add_node(events[1].clone())?;

        let tree = handler.build(events[2].clone())?;

        let expect_tree = ExpectNode::Node{
            metadata: ExpectMetadata(
                NodeMetadataKey{ kind: syntax_kind::expr, offset: 0, len: 24, is_leaf: false },
                NodeMetadata{ edit_state: 0, node_type: NodeType::Node, recovery: None, char_offset: 0, char_len: 18 }
            ),
            children: &[
                ExpectNode::Node { 
                    metadata: ExpectMetadata(
                        NodeMetadataKey{ kind: syntax_kind::term, offset: 0, len: 24, is_leaf: false },
                        NodeMetadata{ edit_state: 0, node_type: NodeType::Node, recovery: None, char_offset: 0, char_len: 18 }
                    ), 
                    children: &[
                        ExpectNode::TokenSet { 
                            metadata: ExpectMetadata(
                                NodeMetadataKey{ kind: syntax_kind::INTEGER, offset: 0, len: 24, is_leaf: false },
                                NodeMetadata{ edit_state: 0, node_type: NodeType::TokenSet, recovery: None, char_offset: 0, char_len: 18 }
                            ), 
                            leading: &[
                                ExpectNode::TokenItem { 
                                    metadata: ExpectMetadata(
                                        NodeMetadataKey{ kind: syntax_kind::COMMENT, offset: 0, len: 21, is_leaf: true },
                                        NodeMetadata{ edit_state: 0, node_type: NodeType::LeadingToken, recovery: None, char_offset: 0, char_len: 15 }
                                    ),
                                    value: "/* 123あいうabc */"
                                },
                                ExpectNode::TokenItem { 
                                    metadata: ExpectMetadata(
                                        NodeMetadataKey{ kind: syntax_kind::SPACE, offset: 21, len: 1, is_leaf: true },
                                        NodeMetadata{ edit_state: 0, node_type: NodeType::LeadingToken, recovery: None, char_offset: 15, char_len: 1 }
                                    ),
                                    value: " "
                                },
                            ], 
                            token: Box::new(ExpectNode::TokenItem { 
                                metadata: ExpectMetadata(
                                    NodeMetadataKey{ kind: syntax_kind::INTEGER, offset: 22, len: 2, is_leaf: true },
                                    NodeMetadata{ edit_state: 0, node_type: NodeType::TokenItem, recovery: None, char_offset: 16, char_len: 2 }
                                ),
                                value: "42"
                            }), 
                            trailing: &[]
                        }
                    ] 
                }
            ]
        };

        verify(&&ActualNode::Node(tree.root()), &expect_tree, 0);
        Ok(())
    }

    #[test]
    fn test_create_token_with_trailing_trivia() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let mut handler = SyntaxTreeBuilder::new(engine.parsing_rules, None);

        let events = vec![
            ParseEvent::Shift { kind: syntax_kind::r#INTEGER, current_state: 0, next_state: 18, edit_state: 0 },
            ParseEvent::Reduce{ kind: syntax_kind::r#term, current_state: 18, next_state: 29, edit_state: 0, pop_count: 1 },
            ParseEvent::Accept{ kind: syntax_kind::r#expr, last_state: 29, edit_state: 0 },
        ];
        let lookahead = Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::r#INTEGER, offset: 0, len: 2, value: Some("42".into()) },
            trailing_trivia: Some(vec![
                ScanEvent{ kind: syntax_kind::r#SPACE, offset: 2, len: 1, value: Some(" ".into()) },
            ]),
        };

        handler.add_token_set(events[0].clone(), Some(&lookahead))?;
        handler.add_node(events[1].clone())?;

        let tree = handler.build(events[2].clone())?;

        let expect_tree = ExpectNode::Node{
            metadata: ExpectMetadata(
                NodeMetadataKey{ kind: syntax_kind::expr, offset: 0, len: 3, is_leaf: false },
                NodeMetadata{ edit_state: 0, node_type: NodeType::Node, recovery: None, char_offset: 0, char_len: 3 }
            ),
            children: &[
                ExpectNode::Node { 
                    metadata: ExpectMetadata(
                        NodeMetadataKey{ kind: syntax_kind::term, offset: 0, len: 3, is_leaf: false },
                        NodeMetadata{ edit_state: 0, node_type: NodeType::Node, recovery: None, char_offset: 0, char_len: 3 }
                    ), 
                    children: &[
                        ExpectNode::TokenSet { 
                            metadata: ExpectMetadata(
                                NodeMetadataKey{ kind: syntax_kind::INTEGER, offset: 0, len: 3, is_leaf: false },
                                NodeMetadata{ edit_state: 0, node_type: NodeType::TokenSet, recovery: None, char_offset: 0, char_len: 3 }
                            ), 
                            leading: &[], 
                            token: Box::new(ExpectNode::TokenItem { 
                                metadata: ExpectMetadata(
                                    NodeMetadataKey{ kind: syntax_kind::INTEGER, offset: 0, len: 2, is_leaf: true },
                                    NodeMetadata{ edit_state: 0, node_type: NodeType::TokenItem, recovery: None, char_offset: 0, char_len: 2 }
                                ),
                                value: "42"
                            }), 
                            trailing: &[
                                ExpectNode::TokenItem { 
                                    metadata: ExpectMetadata(
                                        NodeMetadataKey{ kind: syntax_kind::SPACE, offset: 2, len: 1, is_leaf: true },
                                        NodeMetadata{ edit_state: 0, node_type: NodeType::TrailingToken, recovery: None, char_offset: 2, char_len: 1 }
                                    ),
                                    value: " "
                                },
                            ]
                        }
                    ] 
                }
            ]
        };

        verify(&&ActualNode::Node(tree.root()), &expect_tree, 0);
        Ok(())
    }

    #[test]
    fn test_create_token_with_full_set() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let mut handler = SyntaxTreeBuilder::new(engine.parsing_rules, None);

        let events = vec![
            ParseEvent::Shift { kind: syntax_kind::r#INTEGER, current_state: 0, next_state: 18, edit_state: 0 },
            ParseEvent::Reduce{ kind: syntax_kind::r#term, current_state: 18, next_state: 29, edit_state: 0, pop_count: 1 },
            ParseEvent::Accept{ kind: syntax_kind::r#expr, last_state: 29, edit_state: 0 },
        ];
        let lookahead = Token{
            leading_trivia: Some(vec![
                ScanEvent{ kind: syntax_kind::r#COMMENT, offset: 0, len: 21, value: Some("/* 123あいうabc */".into()) },
            ]),
            main: ScanEvent{ kind: syntax_kind::r#INTEGER, offset: 21, len: 2, value: Some("42".into()) },
            trailing_trivia: Some(vec![
                ScanEvent{ kind: syntax_kind::r#SPACE, offset: 23, len: 1, value: Some(" ".into()) },
            ]),
        };

        handler.add_token_set(events[0].clone(), Some(&lookahead))?;
        handler.add_node(events[1].clone())?;

        let tree = handler.build(events[2].clone())?;

        let expect_tree = ExpectNode::Node{
            metadata: ExpectMetadata(
                NodeMetadataKey{ kind: syntax_kind::expr, offset: 0, len: 24, is_leaf: false },
                NodeMetadata{ edit_state: 0, node_type: NodeType::Node, recovery: None, char_offset: 0, char_len: 18 }
            ),
            children: &[
                ExpectNode::Node { 
                    metadata: ExpectMetadata(
                        NodeMetadataKey{ kind: syntax_kind::term, offset: 0, len: 24, is_leaf: false },
                        NodeMetadata{ edit_state: 0, node_type: NodeType::Node, recovery: None, char_offset: 0, char_len: 18 }
                    ), 
                    children: &[
                        ExpectNode::TokenSet { 
                            metadata: ExpectMetadata(
                                NodeMetadataKey{ kind: syntax_kind::INTEGER, offset: 0, len: 24, is_leaf: false },
                                NodeMetadata{ edit_state: 0, node_type: NodeType::TokenSet, recovery: None, char_offset: 0, char_len: 18 }
                            ), 
                            leading: &[
                                ExpectNode::TokenItem { 
                                    metadata: ExpectMetadata(
                                        NodeMetadataKey{ kind: syntax_kind::COMMENT, offset: 0, len: 21, is_leaf: true },
                                        NodeMetadata{ edit_state: 0, node_type: NodeType::LeadingToken, recovery: None, char_offset: 0, char_len: 15 }
                                    ),
                                    value: "/* 123あいうabc */"
                                },
                            ], 
                            token: Box::new(ExpectNode::TokenItem { 
                                metadata: ExpectMetadata(
                                    NodeMetadataKey{ kind: syntax_kind::INTEGER, offset: 21, len: 2, is_leaf: true },
                                    NodeMetadata{ edit_state: 0, node_type: NodeType::TokenItem, recovery: None, char_offset: 15, char_len: 2 }
                                ),
                                value: "42"
                            }), 
                            trailing: &[
                                ExpectNode::TokenItem { 
                                    metadata: ExpectMetadata(
                                        NodeMetadataKey{ kind: syntax_kind::SPACE, offset: 23, len: 1, is_leaf: true },
                                        NodeMetadata{ edit_state: 0, node_type: NodeType::TrailingToken, recovery: None, char_offset: 17, char_len: 1 }
                                    ),
                                    value: " "
                                },
                            ]
                        }
                    ] 
                }
            ]
        };

        verify(&&ActualNode::Node(tree.root()), &expect_tree, 0);
        Ok(())
    }
}