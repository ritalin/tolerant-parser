use parser_core::{event_dispatcher::ParseEvent, node_handler::{NodeBuildError, SyntaxTreeBuilder}};
use parser_core::support::test_support::*;
    use sqlite_engine::syntax_kind;

mod build_tree_tests {
    use engine_core::scanner_engine::ScanEvent;
    use parser_core::{NodeType, ParseMode, PatchAction};
    use scanner_core::Token;
    use super::*;

    #[test]
    fn test_create_empty_syntax() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let handler = SyntaxTreeBuilder::new(engine.parsing_rules, ParseMode::Full, None);
        let event = ParseEvent::Accept { kind: syntax_kind::r#input, last_state: 0, edit_state: 0 };
        assert_eq!(Err(NodeBuildError::EmptyTree), handler.build(event));
        Ok(())
    }

    #[test]
    fn test_create_eof_only() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let mut handler = SyntaxTreeBuilder::new(engine.parsing_rules, ParseMode::Full, None);
        let event = ParseEvent::Shift { kind: syntax_kind::r#EOF, current_state: 0, next_state: 0, edit_state: 0 };
        assert_eq!(Ok(()), handler.add_kind_token(event));
        
        let event = ParseEvent::Accept { kind: syntax_kind::r#input, last_state: 0, edit_state: 0 };
        let tree = handler.build(event);
        assert!(tree.is_ok());

        let tree = tree.unwrap();

        let expect_tree = &[
            ExpectNode {
                path: vec!["input".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 0, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::Node, patch: PatchAction::None, char_offset: 0, char_len: 0 },
                value: None,
            },
            ExpectNode { 
                path: vec!["input".into(), "EOF".into()],
                meta_key: ExpectMetadataKey { byte_offset: 0, byte_len: 0, is_leaf: false },
                meta_obj: ExpectMetadataValue { edit_state: 0, node_type: NodeType::TokenSet, patch: PatchAction::None, char_offset: 0, char_len: 0 },
                value: None,
            },
            ExpectNode { 
                path: vec!["input".into(), "EOF".into(), "EOF".into()],
                meta_key: ExpectMetadataKey { byte_offset: 0, byte_len: 0, is_leaf: true },
                meta_obj: ExpectMetadataValue { edit_state: 0, node_type: NodeType::TokenItem, patch: PatchAction::None, char_offset: 0, char_len: 0 },
                value: None,
            }
        ];
        verify(tree.root(), expect_tree);
        Ok(())
    }

    #[test]
    fn test_create_token_for_main_only_lookahead() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let mut handler = SyntaxTreeBuilder::new(engine.parsing_rules, ParseMode::Full, None);

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

        let expect_tree = &[
            ExpectNode {
                path: vec!["expr".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 2, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::Node, patch: PatchAction::None, char_offset: 0, char_len: 2 },
                value: None,
            },
            ExpectNode { 
                path: vec!["expr".into(), "term".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 2, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::Node, patch: PatchAction::None, char_offset: 0, char_len: 2 },
                value: None, 
            },
            ExpectNode { 
                path: vec!["expr".into(), "term".into(), "INTEGER".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 2, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TokenSet, patch: PatchAction::None, char_offset: 0, char_len: 2 },
                value: None, 
            },
            ExpectNode { 
                path: vec!["expr".into(), "term".into(), "INTEGER".into(), "INTEGER".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 2, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TokenItem, patch: PatchAction::None, char_offset: 0, char_len: 2 },
                value: Some("42".into()),
            }
        ];

        verify(tree.root(), expect_tree);
        Ok(())
    }

    #[test]
    fn test_create_token_with_leading_trivia() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let mut handler = SyntaxTreeBuilder::new(engine.parsing_rules, ParseMode::Full, None);

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

        let expect_tree = &[
            ExpectNode {
                path: vec!["expr".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 24, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::Node, patch: PatchAction::None, char_offset: 0, char_len: 18 },
                value: None,
            },
            ExpectNode { 
                path: vec!["expr".into(), "term".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 24, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::Node, patch: PatchAction::None, char_offset: 0, char_len: 18 },
                value: None, 
            },
            ExpectNode { 
                path: vec!["expr".into(), "term".into(), "INTEGER".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 24, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TokenSet, patch: PatchAction::None, char_offset: 0, char_len: 18 },
                value: None,
            },
            ExpectNode { 
                path: vec!["expr".into(), "term".into(), "INTEGER".into(), "COMMENT".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 21, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::LeadingToken, patch: PatchAction::None, char_offset: 0, char_len: 15 },
                value: Some("/* 123あいうabc */".into()),
            },
            ExpectNode { 
                path: vec!["expr".into(), "term".into(), "INTEGER".into(), "SPACE".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 21, byte_len: 1, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::LeadingToken, patch: PatchAction::None, char_offset: 15, char_len: 1 },
                value: Some(" ".into()),
            },
            ExpectNode {
                path: vec!["expr".into(), "term".into(), "INTEGER".into(), "INTEGER".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 22, byte_len: 2, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TokenItem, patch: PatchAction::None, char_offset: 16, char_len: 2 },
                value: Some("42".into()),
            }
        ];

        verify(tree.root(), expect_tree);
        Ok(())
    }

    #[test]
    fn test_create_token_with_trailing_trivia() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let mut handler = SyntaxTreeBuilder::new(engine.parsing_rules, ParseMode::Full, None);

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

        let expect_tree =&[
            ExpectNode {
                path: vec!["expr".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 3, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::Node, patch: PatchAction::None, char_offset: 0, char_len: 3 },
                value: None,
            },
            ExpectNode { 
                path: vec!["expr".into(), "term".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 3, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::Node, patch: PatchAction::None, char_offset: 0, char_len: 3 },
                value: None,
            },
            ExpectNode { 
                path: vec!["expr".into(), "term".into(), "INTEGER".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 3, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TokenSet, patch: PatchAction::None, char_offset: 0, char_len: 3 },
                value: None,
            },
            ExpectNode { 
                path: vec!["expr".into(), "term".into(), "INTEGER".into(), "INTEGER".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 2, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TokenItem, patch: PatchAction::None, char_offset: 0, char_len: 2 },
                value: Some("42".into()),
            }, 
            ExpectNode { 
                path: vec!["expr".into(), "term".into(), "INTEGER".into(), "SPACE".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 2, byte_len: 1, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TrailingToken, patch: PatchAction::None, char_offset: 2, char_len: 1 },
                value: Some(" ".into()),
            }
        ];

        verify(tree.root(), expect_tree);
        Ok(())
    }

    #[test]
    fn test_create_token_with_full_set() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let mut handler = SyntaxTreeBuilder::new(engine.parsing_rules, ParseMode::Full, None);

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

        let expect_tree = &[
            ExpectNode {
                path: vec!["expr".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 24, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::Node, patch: PatchAction::None, char_offset: 0, char_len: 18 },
                value: None,
            },
            ExpectNode { 
                path: vec!["expr".into(), "term".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 24, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::Node, patch: PatchAction::None, char_offset: 0, char_len: 18 },
                value: None,
            },
            ExpectNode { 
                path: vec!["expr".into(), "term".into(), "INTEGER".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 24, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TokenSet, patch: PatchAction::None, char_offset: 0, char_len: 18 },
                value: None,
            },
            ExpectNode { 
                path: vec!["expr".into(), "term".into(), "INTEGER".into(), "COMMENT".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 21, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::LeadingToken, patch: PatchAction::None, char_offset: 0, char_len: 15 },
                value: Some("/* 123あいうabc */".into()),
            },
            ExpectNode { 
                path: vec!["expr".into(), "term".into(), "INTEGER".into(), "INTEGER".into()],
                meta_key:  ExpectMetadataKey{ byte_offset: 21, byte_len: 2, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TokenItem, patch: PatchAction::None, char_offset: 15, char_len: 2 },
                value: Some("42".into()),
            }, 
            ExpectNode { 
                path: vec!["expr".into(), "term".into(), "INTEGER".into(), "SPACE".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 23, byte_len: 1, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TrailingToken, patch: PatchAction::None, char_offset: 17, char_len: 1 },
                value: Some(" ".into()),
            },
        ];

        verify(tree.root(), expect_tree);
        Ok(())
    }

    #[test]
    fn test_create_patch_node_for_deleting_recovery() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let mut handler = SyntaxTreeBuilder::new(engine.parsing_rules, ParseMode::Full, None);

        let lookaheads = vec![
            Token{
                leading_trivia: Some(vec![
                    ScanEvent{ kind: syntax_kind::SPACE, offset: 0, len: 4, value: Some("    ".into()) }
                ]),
                main: ScanEvent { kind: syntax_kind::BLOB, offset: 4, len: 6, value: Some("x'abc'".into()) },
                trailing_trivia: None
            },
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 10, len: 2, value: Some("42".into()) },
                trailing_trivia: None,
            }

        ];
        let events = vec![
            ParseEvent::PatchDrop { kind: syntax_kind::BLOB, current_state: 214, next_state: 214, edit_state: 214 },
            ParseEvent::Shift { kind: syntax_kind::INTEGER, current_state: 214, next_state: 122, edit_state: 214 },
            ParseEvent::Reduce { kind: syntax_kind::term, current_state: 122, next_state: 128, edit_state: 214, pop_count: 1 },
            ParseEvent::Accept{ kind: syntax_kind::r#expr, last_state: 29, edit_state: 0 },
        ];

        handler.add_invisible_token_set(events[0].clone(), lookaheads.get(0))?;
        handler.add_token_set(events[1].clone(), lookaheads.get(1))?;
        handler.add_node(events[2].clone())?;

        let tree = handler.build(events[3].clone())?;

        let expect_tree = &[
            ExpectNode {
                path: vec!["expr".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 12, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::Node, patch: PatchAction::None, char_offset: 0, char_len: 12 },
                value: None,
            },
            ExpectNode {
                path: vec!["expr".into(), "term".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 12, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 214, node_type: NodeType::Node, patch: PatchAction::None, char_offset: 0, char_len: 12 },
                value: None,
            },
            ExpectNode { 
                path: vec!["expr".into(), "term".into(), "BLOB".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 10, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 214, node_type: NodeType::TokenSet, patch: PatchAction::Delete, char_offset: 0, char_len: 10 },
                value: None
            },
            ExpectNode { 
                path: vec!["expr".into(), "term".into(), "BLOB".into(), "SPACE".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 4, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 214, node_type: NodeType::LeadingToken, patch: PatchAction::Delete, char_offset: 0, char_len: 4 },
                value: Some("    ".into()),
            },
            ExpectNode { 
                path: vec!["expr".into(), "term".into(), "BLOB".into(), "BLOB".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 4, byte_len: 6, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 214, node_type: NodeType::TokenItem, patch: PatchAction::Delete, char_offset: 4, char_len: 6 },
                value: Some("x'abc'".into()),
            }, 
            ExpectNode { 
                path: vec!["expr".into(), "term".into(), "INTEGER".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 10, byte_len: 2, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 214, node_type: NodeType::TokenSet, patch: PatchAction::None, char_offset: 10, char_len: 2 },
                value: None,
            },
            ExpectNode { 
                path: vec!["expr".into(), "term".into(), "INTEGER".into(), "INTEGER".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 10, byte_len: 2, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 214, node_type: NodeType::TokenItem, patch: PatchAction::None, char_offset: 10, char_len: 2 },
                value: Some("42".into()),
            }, 
        ];

        verify(tree.root(), expect_tree);
        Ok(())
    }

    #[test]
    fn test_create_patch_node_for_shifting_recovery() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let mut handler = SyntaxTreeBuilder::new(engine.parsing_rules, ParseMode::Full, None);

        let lookaheads = vec![
            Token{
                leading_trivia: None,
                main: ScanEvent { kind: syntax_kind::INTEGER, offset: 0, len: 3, value: Some("101".into()) },
                trailing_trivia: Some(vec![
                    ScanEvent{ kind: syntax_kind::SPACE, offset: 3, len: 1, value: Some(" ".into()) }
                ])
            },
            Token{
                leading_trivia: Some(vec![
                    ScanEvent{ kind: syntax_kind::COMMENT, offset: 4, len: 24, value: Some("/* where is operator? */".into()) }
                ]),
                main: ScanEvent { kind: syntax_kind::INTEGER, offset: 28, len: 2, value: Some("49".into()) },
                trailing_trivia: Some(vec![
                    ScanEvent{ kind: syntax_kind::SPACE, offset: 30, len: 3, value: Some("   ".into()) }
                ]),
            }
        ];
        let events = vec![
            ParseEvent::Shift { kind: syntax_kind::INTEGER, current_state: 100, next_state: 122, edit_state: 100 },
            ParseEvent::PatchReduce{ kind: syntax_kind::term, current_state: 122, next_state: 128, edit_state: 238, pop_count: 1 },
            ParseEvent::PatchReduce{ kind: syntax_kind::expr, current_state: 128, next_state: 361, edit_state: 238, pop_count: 1 },
            ParseEvent::PatchShift{ kind: syntax_kind::STAR, current_state: 361, next_state: 214, edit_state: 361 },
            ParseEvent::Shift{ kind: syntax_kind::STAR, current_state: 214, next_state: 122, edit_state: 214 },
            ParseEvent::Accept{ kind: syntax_kind::columnlist, last_state: 29, edit_state: 0 },
        ];

        handler.add_token_set(events[0].clone(), lookaheads.get(0))?;
        handler.add_node(events[1].clone())?;
        handler.add_node(events[2].clone())?;
        handler.add_patch_shift_token_set(events[3].clone())?;
        handler.add_token_set(events[4].clone(), lookaheads.get(1))?;

        let tree = handler.build(events[5].clone())?;

        let expect_tree = &[
            ExpectNode{
                path: vec!["columnlist".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 33, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::Node, patch: PatchAction::None, char_offset: 0, char_len: 33 },
                value: None,
            },
            ExpectNode{
                path: vec!["columnlist".into(), "expr".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 4, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 238, node_type: NodeType::Node, patch: PatchAction::Shift, char_offset: 0, char_len: 4 },
                value: None,
            },
            ExpectNode{
                path: vec!["columnlist".into(), "expr".into(), "term".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 4, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 238, node_type: NodeType::Node, patch: PatchAction::Shift, char_offset: 0, char_len: 4 },
                value: None,
            },
            ExpectNode { 
                path: vec!["columnlist".into(), "expr".into(), "term".into(), "INTEGER".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 4, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 100, node_type: NodeType::TokenSet, patch: PatchAction::None, char_offset: 0, char_len: 4 },
                value: None,
            },
            ExpectNode { 
                path: vec!["columnlist".into(), "expr".into(), "term".into(), "INTEGER".into(), "INTEGER".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 3, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 100, node_type: NodeType::TokenItem, patch: PatchAction::None, char_offset: 0, char_len: 3 },
                value: Some("101".into()),
            },
            ExpectNode {
                path: vec!["columnlist".into(), "expr".into(), "term".into(), "INTEGER".into(), "SPACE".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 3, byte_len: 1, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 100, node_type: NodeType::TrailingToken, patch: PatchAction::None, char_offset: 3, char_len: 1 },
                value: Some(" ".into()),
            },
            ExpectNode { 
                path: vec!["columnlist".into(), "STAR".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 4, byte_len: 0, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 361, node_type: NodeType::TokenSet, patch: PatchAction::Shift, char_offset: 4, char_len: 0 },
                value: None,
            },
            ExpectNode { 
                path: vec!["columnlist".into(), "STAR".into(), "STAR".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 4, byte_len: 0, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 361, node_type: NodeType::TokenItem, patch: PatchAction::Shift, char_offset: 4, char_len: 0 },
                value: None,
            },
            ExpectNode { 
                path: vec!["columnlist".into(), "INTEGER".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 4, byte_len: 29, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 214, node_type: NodeType::TokenSet, patch: PatchAction::None, char_offset: 4, char_len: 29 },
                value: None,
            },
            ExpectNode { 
                path: vec!["columnlist".into(), "INTEGER".into(), "COMMENT".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 4, byte_len: 24, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 214, node_type: NodeType::LeadingToken, patch: PatchAction::None, char_offset: 4, char_len: 24 },
                value: Some("/* where is operator? */".into()),
            },
            ExpectNode { 
                path: vec!["columnlist".into(), "INTEGER".into(), "INTEGER".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 28, byte_len: 2, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 214, node_type: NodeType::TokenItem, patch: PatchAction::None, char_offset: 28, char_len: 2 },
                value: Some("49".into()),
            },
            ExpectNode { 
                path: vec!["columnlist".into(), "INTEGER".into(), "SPACE".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 30, byte_len: 3, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 214, node_type: NodeType::TrailingToken, patch: PatchAction::None, char_offset: 30, char_len: 3 },
                value: Some("   ".into()),
            },
        ];

        verify(tree.root(), expect_tree);
        Ok(())
    }

    #[test]
    fn test_create_patch_node_for_recovery_failed() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let mut handler = SyntaxTreeBuilder::new(engine.parsing_rules, ParseMode::Full, None);

        let lookaheads = vec![
            Token{
                leading_trivia: None,
                main: ScanEvent { kind: syntax_kind::INTEGER, offset: 0, len: 3, value: Some("101".into()) },
                trailing_trivia: Some(vec![
                    ScanEvent{ kind: syntax_kind::SPACE, offset: 3, len: 1, value: Some(" ".into()) }
                ])
            },
            Token{
                leading_trivia: Some(vec![
                    ScanEvent{ kind: syntax_kind::COMMENT, offset: 4, len: 24, value: Some("/* where is operator? */".into()) }
                ]),
                main: ScanEvent { kind: syntax_kind::INTEGER, offset: 28, len: 2, value: Some("49".into()) },
                trailing_trivia: Some(vec![
                    ScanEvent{ kind: syntax_kind::SPACE, offset: 30, len: 3, value: Some("   ".into()) }
                ]),
            }
        ];
        let events = vec![
            ParseEvent::Invalid { kind: syntax_kind::INTEGER, current_state: 122, edit_state: 0 },
            ParseEvent::Invalid { kind: syntax_kind::INTEGER, current_state: 122, edit_state: 0 },
            ParseEvent::Accept{ kind: syntax_kind::expr, last_state: 29, edit_state: 0 },
        ];

        handler.add_token_set(events[0].clone(), lookaheads.get(0))?;
        handler.add_token_set(events[1].clone(), lookaheads.get(1))?;

        let tree = handler.build(events[2].clone())?;

        let expect_tree = &[
            ExpectNode{
                path: vec!["expr".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 33, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::Node, patch: PatchAction::None, char_offset: 0, char_len: 33 },
                value: None,
            },
            ExpectNode { 
                path: vec!["expr".into(), "INTEGER".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 4, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TokenSet, patch: PatchAction::Invalid, char_offset: 0, char_len: 4 },
                value: None,
            },
            ExpectNode { 
                path: vec!["expr".into(), "INTEGER".into(), "INTEGER".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 0, byte_len: 3, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TokenItem, patch: PatchAction::Invalid, char_offset: 0, char_len: 3 },
                value: Some("101".into()),
            },
            ExpectNode { 
                path: vec!["expr".into(), "INTEGER".into(), "SPACE".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 3, byte_len: 1, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TrailingToken, patch: PatchAction::Invalid, char_offset: 3, char_len: 1 },
                value: Some(" ".into()),
            },
            ExpectNode { 
                path: vec!["expr".into(), "INTEGER".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 4, byte_len: 29, is_leaf: false },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TokenSet, patch: PatchAction::Invalid, char_offset: 4, char_len: 29 },
                value: None,
            },
            ExpectNode { 
                path: vec!["expr".into(), "INTEGER".into(), "COMMENT".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 4, byte_len: 24, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::LeadingToken, patch: PatchAction::Invalid, char_offset: 4, char_len: 24 },
                value: Some("/* where is operator? */".into()),
            },
            ExpectNode { 
                path: vec!["expr".into(), "INTEGER".into(), "INTEGER".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 28, byte_len: 2, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TokenItem, patch: PatchAction::Invalid, char_offset: 28, char_len: 2 },
                value: Some("49".into()),
            },
            ExpectNode { 
                path: vec!["expr".into(), "INTEGER".into(), "SPACE".into()],
                meta_key: ExpectMetadataKey{ byte_offset: 30, byte_len: 3, is_leaf: true },
                meta_obj: ExpectMetadataValue{ edit_state: 0, node_type: NodeType::TrailingToken, patch: PatchAction::Invalid, char_offset: 30, char_len: 3 },
                value: Some("   ".into()),
            },
        ];

        verify(tree.root(), expect_tree);
        Ok(())
    }
}