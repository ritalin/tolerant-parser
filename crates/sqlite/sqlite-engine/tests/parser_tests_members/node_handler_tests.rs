use parser_core::{event_dispatcher::ParseEvent, node_handler::{NodeBuildError, SyntaxTreeBuilder}};
use sqlite_engine::syntax_kind;

use crate::test_support::*;

mod build_tree_tests {
    use engine_core::scanner_engine::ScanEvent;
    use parser_core::NodeType;
    use scanner_core::Token;
    use super::*;

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
        let parsing_rules = engine.parsing_rules;
        let mut handler = SyntaxTreeBuilder::new(engine.parsing_rules, None);
        let event = ParseEvent::Shift { kind: syntax_kind::r#EOF, current_state: 0, next_state: 0, edit_state: 0 };
        assert_eq!(Ok(()), handler.add_kind_token(event));
        
        let event = ParseEvent::Accept { kind: syntax_kind::r#input, last_state: 0, edit_state: 0 };
        let tree = handler.build(event);
        assert!(tree.is_ok());

        let tree = tree.unwrap();

        let expect_tree = ExpectNode::Node{
            metadata: ExpectMetadata(
                ExpectMetadataKey{ kind_id: syntax_kind::input.id, kind_name: "".into(), byte_offset: 0, byte_len: 0, is_leaf: false },
                ExpectMetadataValue{ edit_state: 0, node_type: NodeType::Node, recovery: None, char_offset: 0, char_len: 0 },
            ),
            children: vec![
                ExpectNode::TokenSet { 
                    metadata: ExpectMetadata(
                        ExpectMetadataKey { kind_id: syntax_kind::r#EOF.id, kind_name: "".into(), byte_offset: 0, byte_len: 0, is_leaf: false },
                        ExpectMetadataValue { edit_state: 0, node_type: NodeType::TokenSet, recovery: None, char_offset: 0, char_len: 0 },
                    ),
                    leading: vec![],
                    token: Box::new(ExpectNode::TokenItem { 
                        metadata: ExpectMetadata(
                            ExpectMetadataKey { kind_id: syntax_kind::r#EOF.id, kind_name: "".into(), byte_offset: 0, byte_len: 0, is_leaf: true },
                            ExpectMetadataValue { edit_state: 0, node_type: NodeType::TokenItem, recovery: None, char_offset: 0, char_len: 0 },
                        ),
                        value: "".into(),
                    }),
                    trailing: vec![],
                }
            ]
        };
        verify(&&ActualNode::Node(tree.root()), &expect_tree, parsing_rules, 0);
        Ok(())
    }

    #[test]
    fn test_create_token_for_main_only_lookahead() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let parsing_rules = engine.parsing_rules;
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
                ExpectMetadataKey{ kind_id: syntax_kind::expr.id, byte_offset: 0, byte_len: 2, is_leaf: false, kind_name: "".into() },
                ExpectMetadataValue{ edit_state: 0, node_type: NodeType::Node, recovery: None, char_offset: 0, char_len: 2 },
            ),
            children: vec![
                ExpectNode::Node { 
                    metadata: ExpectMetadata(
                        ExpectMetadataKey{ kind_id: syntax_kind::term.id, byte_offset: 0, byte_len: 2, is_leaf: false, kind_name: "".into() },
                        ExpectMetadataValue{ edit_state: 0, node_type: NodeType::Node, recovery: None, char_offset: 0, char_len: 2 },
                    ), 
                    children: vec![
                        ExpectNode::TokenSet { 
                            metadata: ExpectMetadata(
                                ExpectMetadataKey{ kind_id: syntax_kind::INTEGER.id, byte_offset: 0, byte_len: 2, is_leaf: false, kind_name: "".into() },
                                ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TokenSet, recovery: None, char_offset: 0, char_len: 2 },
                            ), 
                            leading: vec![], 
                            token: Box::new(ExpectNode::TokenItem { 
                                metadata: ExpectMetadata(
                                    ExpectMetadataKey{ kind_id: syntax_kind::INTEGER.id, byte_offset: 0, byte_len: 2, is_leaf: true, kind_name: "".into() },
                                    ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TokenItem, recovery: None, char_offset: 0, char_len: 2 },
                                ),
                                value: "42".into()
                            }), 
                            trailing: vec![]
                        }
                    ] 
                }
            ]
        };

        verify(&&ActualNode::Node(tree.root()), &expect_tree, parsing_rules, 0);
        Ok(())
    }

    #[test]
    fn test_create_token_with_leading_trivia() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let parsing_rules = engine.parsing_rules;
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
                ExpectMetadataKey{ kind_id: syntax_kind::expr.id, byte_offset: 0, byte_len: 24, is_leaf: false, kind_name: "".into() },
                ExpectMetadataValue{ edit_state: 0, node_type: NodeType::Node, recovery: None, char_offset: 0, char_len: 18 },
            ),
            children: vec![
                ExpectNode::Node { 
                    metadata: ExpectMetadata(
                        ExpectMetadataKey{ kind_id: syntax_kind::term.id, byte_offset: 0, byte_len: 24, is_leaf: false, kind_name: "".into() },
                        ExpectMetadataValue{ edit_state: 0, node_type: NodeType::Node, recovery: None, char_offset: 0, char_len: 18 },
                    ), 
                    children: vec![
                        ExpectNode::TokenSet { 
                            metadata: ExpectMetadata(
                                ExpectMetadataKey{ kind_id: syntax_kind::INTEGER.id, byte_offset: 0, byte_len: 24, is_leaf: false, kind_name: "".into() },
                                ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TokenSet, recovery: None, char_offset: 0, char_len: 18 },
                            ), 
                            leading: vec![
                                ExpectNode::TokenItem { 
                                    metadata: ExpectMetadata(
                                        ExpectMetadataKey{ kind_id: syntax_kind::COMMENT.id, byte_offset: 0, byte_len: 21, is_leaf: true, kind_name: "".into() },
                                        ExpectMetadataValue{ edit_state: 0, node_type: NodeType::LeadingToken, recovery: None, char_offset: 0, char_len: 15 },
                                    ),
                                    value: "/* 123あいうabc */".into()
                                },
                                ExpectNode::TokenItem { 
                                    metadata: ExpectMetadata(
                                        ExpectMetadataKey{ kind_id: syntax_kind::SPACE.id, byte_offset: 21, byte_len: 1, is_leaf: true, kind_name: "".into() },
                                        ExpectMetadataValue{ edit_state: 0, node_type: NodeType::LeadingToken, recovery: None, char_offset: 15, char_len: 1 },
                                    ),
                                    value: " ".into()
                                },
                            ], 
                            token: Box::new(ExpectNode::TokenItem { 
                                metadata: ExpectMetadata(
                                    ExpectMetadataKey{ kind_id: syntax_kind::INTEGER.id, byte_offset: 22, byte_len: 2, is_leaf: true, kind_name: "".into() },
                                    ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TokenItem, recovery: None, char_offset: 16, char_len: 2 },
                                ),
                                value: "42".into()
                            }), 
                            trailing: vec![]
                        }
                    ] 
                }
            ]
        };

        verify(&&ActualNode::Node(tree.root()), &expect_tree, parsing_rules, 0);
        Ok(())
    }

    #[test]
    fn test_create_token_with_trailing_trivia() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let parsing_rules = engine.parsing_rules;
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
                ExpectMetadataKey{ kind_id: syntax_kind::expr.id, byte_offset: 0, byte_len: 3, is_leaf: false, kind_name: "".into() },
                ExpectMetadataValue{ edit_state: 0, node_type: NodeType::Node, recovery: None, char_offset: 0, char_len: 3 },
            ),
            children: vec![
                ExpectNode::Node { 
                    metadata: ExpectMetadata(
                        ExpectMetadataKey{ kind_id: syntax_kind::term.id, byte_offset: 0, byte_len: 3, is_leaf: false, kind_name: "".into() },
                        ExpectMetadataValue{ edit_state: 0, node_type: NodeType::Node, recovery: None, char_offset: 0, char_len: 3 },
                    ), 
                    children: vec![
                        ExpectNode::TokenSet { 
                            metadata: ExpectMetadata(
                                ExpectMetadataKey{ kind_id: syntax_kind::INTEGER.id, byte_offset: 0, byte_len: 3, is_leaf: false, kind_name: "".into() },
                                ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TokenSet, recovery: None, char_offset: 0, char_len: 3 },
                            ), 
                            leading: vec![], 
                            token: Box::new(ExpectNode::TokenItem { 
                                metadata: ExpectMetadata(
                                    ExpectMetadataKey{ kind_id: syntax_kind::INTEGER.id, byte_offset: 0, byte_len: 2, is_leaf: true, kind_name: "".into() },
                                    ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TokenItem, recovery: None, char_offset: 0, char_len: 2 },
                                ),
                                value: "42".into()
                            }), 
                            trailing: vec![
                                ExpectNode::TokenItem { 
                                    metadata: ExpectMetadata(
                                        ExpectMetadataKey{ kind_id: syntax_kind::SPACE.id, byte_offset: 2, byte_len: 1, is_leaf: true, kind_name: "".into() },
                                        ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TrailingToken, recovery: None, char_offset: 2, char_len: 1 },
                                    ),
                                    value: " ".into()
                                },
                            ]
                        }
                    ] 
                }
            ]
        };

        verify(&&ActualNode::Node(tree.root()), &expect_tree, parsing_rules, 0);
        Ok(())
    }

    #[test]
    fn test_create_token_with_full_set() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let parsing_rules = engine.parsing_rules;
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
                ExpectMetadataKey{ kind_id: syntax_kind::expr.id, byte_offset: 0, byte_len: 24, is_leaf: false, kind_name: "".into() },
                ExpectMetadataValue{ edit_state: 0, node_type: NodeType::Node, recovery: None, char_offset: 0, char_len: 18 },
            ),
            children: vec![
                ExpectNode::Node { 
                    metadata: ExpectMetadata(
                        ExpectMetadataKey{ kind_id: syntax_kind::term.id, byte_offset: 0, byte_len: 24, is_leaf: false, kind_name: "".into() },
                        ExpectMetadataValue{ edit_state: 0, node_type: NodeType::Node, recovery: None, char_offset: 0, char_len: 18 },
                    ), 
                    children: vec![
                        ExpectNode::TokenSet { 
                            metadata: ExpectMetadata(
                                ExpectMetadataKey{ kind_id: syntax_kind::INTEGER.id, byte_offset: 0, byte_len: 24, is_leaf: false, kind_name: "".into() },
                                ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TokenSet, recovery: None, char_offset: 0, char_len: 18 },
                            ), 
                            leading: vec![
                                ExpectNode::TokenItem { 
                                    metadata: ExpectMetadata(
                                        ExpectMetadataKey{ kind_id: syntax_kind::COMMENT.id, byte_offset: 0, byte_len: 21, is_leaf: true, kind_name: "".into() },
                                        ExpectMetadataValue{ edit_state: 0, node_type: NodeType::LeadingToken, recovery: None, char_offset: 0, char_len: 15 },
                                    ),
                                    value: "/* 123あいうabc */".into()
                                },
                            ], 
                            token: Box::new(ExpectNode::TokenItem { 
                                metadata: ExpectMetadata(
                                    ExpectMetadataKey{ kind_id: syntax_kind::INTEGER.id, byte_offset: 21, byte_len: 2, is_leaf: true, kind_name: "".into() },
                                    ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TokenItem, recovery: None, char_offset: 15, char_len: 2 },
                                ),
                                value: "42".into()
                            }), 
                            trailing: vec![
                                ExpectNode::TokenItem { 
                                    metadata: ExpectMetadata(
                                        ExpectMetadataKey{ kind_id: syntax_kind::SPACE.id, byte_offset: 23, byte_len: 1, is_leaf: true, kind_name: "".into() },
                                        ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TrailingToken, recovery: None, char_offset: 17, char_len: 1 },
                                    ),
                                    value: " ".into()
                                },
                            ]
                        }
                    ] 
                }
            ]
        };

        verify(&&ActualNode::Node(tree.root()), &expect_tree, parsing_rules, 0);
        Ok(())
    }
}