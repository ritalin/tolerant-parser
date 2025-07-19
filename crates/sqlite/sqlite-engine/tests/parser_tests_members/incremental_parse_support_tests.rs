
use tolerant_parser_sdk::core::engine_core::{SyntaxKind, scanner_engine::CaseSensitivity};
use tolerant_parser_sdk::core::parser_core::{incremental::EditScope, syntax_tree::SyntaxNode, Parser, ParserConfig, ParseMode, RecoveryPenalty};
use sqlite_engine::syntax_kind;

mod expand_region_tests {
    use tolerant_parser_sdk::core::parser_core::{self, syntax_tree::MetadataAccess};

    use super::*;

    fn extend_to_neighbors(scope: std::ops::Range<usize>, root: Option<&SyntaxNode>, except_kind: SyntaxKind) -> std::ops::Range<usize> {
        let Some(root) = root else { return scope; };

        let adjusted_range = parser_core::incremental::support::intersect_edit_range(&scope, &root.metadata_key().byte_range());

        let gardener = parser_core::incremental::support::TreeGardener::as_subtree(root);
        let anscestor = gardener.common_anscestor(
            gardener.pick_token(adjusted_range.start),
            gardener.pick_token(adjusted_range.end),
            except_kind
        ).unwrap();
        let range: std::ops::Range<usize> = anscestor.node.text_range().into();

        (range.start + anscestor.metadata_entry.global_offset.of_byte)..(range.end + anscestor.metadata_entry.global_offset.of_byte)
    }

    #[test]
    fn test_extend_to_neighbor_without_node() -> Result<(), anyhow::Error> {
        let scope = EditScope {
            start_char_offset: 11,
            old_char_len: 23,
            new_char_len: 5,
            text: "Hello".into()
        };

        let new_scope = extend_to_neighbors(scope.old_char_range(), None, syntax_kind::SEMI);
        assert_eq!(scope.old_char_range(), new_scope);
        Ok(())
    }

    #[test]
    fn test_extend_to_neighbor_for_fitting_node() -> Result<(), anyhow::Error> {
        let source = "SELECT 101 AS x FROM foo u;";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 27,
            new_char_len: 27,
            text: source.to_string(),
        };
        let new_scope = extend_to_neighbors(scope.old_char_range(), Some(&tree.root()), syntax_kind::SEMI);
        assert_eq!(0..27, new_scope);
        Ok(())
    }

    #[test]
    fn test_extend_to_neighbor_for_overall_node() -> Result<(), anyhow::Error> {
        let source = "SELECT 42;SELECT 101 AS x FROM foo u;SELECT a.b FROM bar;";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 7,
            old_char_len: 33,
            new_char_len: 28,
            text: "42;SELECT 101 FROM foo u;SEL".into(),
        };
        let new_scope = extend_to_neighbors(
            scope.old_char_range(), 
            tree.root().nth_child(1).unwrap().to_node().as_ref(),
            syntax_kind::SEMI
        );
        assert_eq!(10..37, new_scope);
        Ok(())
    }

    #[test]
    fn test_extend_to_neighbor_for_inside_node() -> Result<(), anyhow::Error> {
        let source = "SELECT 42;SELECT 101 AS x FROM foo u;SELECT a.xyz AS v FROM bar;";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 45, // DOT
            old_char_len: 4,
            new_char_len: 3,
            text: ".ab".into(),
        };
        let new_scope = extend_to_neighbors(
            scope.old_char_range(),
            tree.root().nth_child(2).unwrap().to_node().as_ref(),
            syntax_kind::SEMI
        );
        assert_eq!(44..55, new_scope);
        Ok(())
    }

    #[test]
    fn test_extend_to_neighbor_cross_over_2_nodes() -> Result<(), anyhow::Error> {
        let source = "SELECT 42;SELECT 101 AS x FROM foo u;SELECT a.xyz AS v FROM bar;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 32,
            old_char_len: 10,
            new_char_len: 13,
            text: "ooooo u;SELEC".into(),
        };

        'left_hand: {
            let new_scope = extend_to_neighbors(
                scope.old_char_range(),
                tree.root().nth_child(1).unwrap().to_node().as_ref(),
                syntax_kind::SEMI
            );
            assert_eq!(10..37, new_scope);
            break 'left_hand;
        }
        'right_hand: {
            let new_scope = extend_to_neighbors(
                scope.old_char_range(),
                tree.root().nth_child(2).unwrap().to_node().as_ref(),
                syntax_kind::SEMI
            );
            assert_eq!(37..63, new_scope);
            break 'right_hand;
        }
        Ok(())
    }
}

mod edit_hint_init_tests {
    use tolerant_parser_sdk::core::parser_core::incremental::edit_hint::EditHint;
    use super::*;

    #[test]
    fn test_init_hint_edit_for_update() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 2;SELECT 3;SELECT 4;SELECT 5;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 9,
            old_char_len: 18,
            new_char_len: 18,
            text: "SELECT 4;SELECT 5;".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());

        assert_eq!(vec![1, 2], hint.statements.into_iter().map(|stmt| stmt.into_raw().index()).collect::<Vec<_>>());
        assert_eq!(vec![0], hint.precedings.into_iter().flatten().map(|node| node.into_raw().index()).collect::<Vec<_>>());
        assert_eq!(vec![3, 4, 5], hint.followings.into_iter().flatten().map(|node| node.into_raw().index()).collect::<Vec<_>>());
        Ok(())
    }

    #[test]
    fn test_init_hint_edit_for_update_without_statement() -> Result<(), anyhow::Error> {
        let source = "";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 10,
            text: "SELECT 42;".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());

        assert_eq!(Vec::<usize>::new(), hint.statements.into_iter().map(|stmt| stmt.into_raw().index()).collect::<Vec<_>>());
        assert_eq!(Vec::<usize>::new(), hint.precedings.into_iter().flatten().map(|node| node.into_raw().index()).collect::<Vec<_>>());
        assert_eq!(vec![0], hint.followings.into_iter().flatten().map(|node| node.into_raw().index()).collect::<Vec<_>>());
        Ok(())
    }

    #[test]
    fn test_init_hint_edit_for_append() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 2;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 18,
            old_char_len: 0,
            new_char_len: 9,
            text: "SELECT 3;".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());

        assert_eq!(Vec::<usize>::new(), hint.statements.into_iter().map(|stmt| stmt.into_raw().index()).collect::<Vec<_>>());
        assert_eq!(vec![1, 0], hint.precedings.into_iter().flatten().map(|node| node.into_raw().index()).collect::<Vec<_>>());
        assert_eq!(vec![2], hint.followings.into_iter().flatten().map(|node| node.into_raw().index()).collect::<Vec<_>>());
        Ok(())
    }

    #[test]
    fn test_init_hint_edit_for_prepend() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 2;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 22,
            text: "SELECT 0;".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());

        assert_eq!(Vec::<usize>::new(), hint.statements.into_iter().map(|stmt| stmt.into_raw().index()).collect::<Vec<_>>());
        assert_eq!(Vec::<usize>::new(), hint.precedings.into_iter().flatten().map(|node| node.into_raw().index()).collect::<Vec<_>>());
        assert_eq!(vec![0, 1, 2], hint.followings.into_iter().flatten().map(|node| node.into_raw().index()).collect::<Vec<_>>());
        Ok(())
    }

    #[test]
    fn test_init_hint_edit_for_insert() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 2;SELECT 3;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 18,
            old_char_len: 0,
            new_char_len: 10,
            text: "SELECT 42;".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());

        assert_eq!(vec![1, 0], hint.precedings.into_iter().flatten().map(|node| node.into_raw().index()).collect::<Vec<_>>());
        assert_eq!(Vec::<usize>::new(), hint.statements.into_iter().map(|stmt| stmt.into_raw().index()).collect::<Vec<_>>());
        assert_eq!(vec![2, 3], hint.followings.into_iter().flatten().map(|node| node.into_raw().index()).collect::<Vec<_>>());
        Ok(())
    }
}

mod edit_hint_reconcile_tests {
    use tolerant_parser_sdk::core::{engine_core::scanner_engine::ScanEvent, parser_core::incremental::edit_hint::EditHint, scanner_core::{iter::StatementScannerType, ScannerAccess, Token}};
    use super::*;

    #[test]
    fn test_append_statement() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 9,
            old_char_len: 0,
            new_char_len: 9,
            text: "SELECT 2;".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let mut scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;

        'scanner: {
            let scanner = scanners.next();
            assert_eq!(true, scanner.is_some());

            let scanner = scanner.unwrap();
            assert_eq!(StatementScannerType::Statement, scanner.scanner_type());
            assert_eq!(0..9, scanner.scan_range());

            let mut scanner_view = scanner.as_view(..);
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::SELECT, offset: 0, len: 6, value: Some("SELECT".into()) },
                    trailing_trivia: Some(vec![ScanEvent{ kind: syntax_kind::SPACE, offset: 6, len: 1, value: Some(" ".into()) }]),
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 7, len: 1, value: Some("1".into()) },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::SEMI, offset: 8, len: 1, value: Some(";".into()) },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            break 'scanner;
        }
        'scanner: {
            let scanner = scanners.next();
            assert_eq!(true, scanner.is_some());

            let scanner = scanner.unwrap();
            assert_eq!(StatementScannerType::Statement, scanner.scanner_type());
            assert_eq!(9..18, scanner.scan_range());

            let mut scanner_view = scanner.as_view(..);
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::SELECT, offset: 9, len: 6, value: Some("SELECT".into()) },
                    trailing_trivia: Some(vec![ScanEvent{ kind: syntax_kind::SPACE, offset: 15, len: 1, value: Some(" ".into()) }]),
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 16, len: 1, value: Some("2".into()) },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::SEMI, offset: 17, len: 1, value: Some(";".into()) },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            break 'scanner;
        }
        'scanner: {
            let scanner = scanners.next();
            assert_eq!(true, scanner.is_some());

            let scanner = scanner.unwrap();
            assert_eq!(StatementScannerType::Eof, scanner.scanner_type());
            assert_eq!(18..18, scanner.scan_range());

            let mut scanner_view = scanner.as_view(..);
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::EOF, offset: 18, len: 0, value: None },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            break 'scanner;
        }

        'scanner: {
            let scanner = scanners.next();
            assert_eq!(false, scanner.is_some());
            break 'scanner;
        }

        Ok(())
    }

    #[test]
    fn test_prepend_statement() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 9,
            text: "SELECT 0;".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let mut scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;

        'scanner: {
            let scanner = scanners.next();
            assert_eq!(true, scanner.is_some());

            let scanner = scanner.unwrap();
            assert_eq!(StatementScannerType::Statement, scanner.scanner_type());
            assert_eq!(0..9, scanner.scan_range());

            let mut scanner_view = scanner.as_view(..);
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::SELECT, offset: 0, len: 6, value: Some("SELECT".into()) },
                    trailing_trivia: Some(vec![ScanEvent{ kind: syntax_kind::SPACE, offset: 6, len: 1, value: Some(" ".into()) }]),
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 7, len: 1, value: Some("0".into()) },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::SEMI, offset: 8, len: 1, value: Some(";".into()) },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            break 'scanner;
        }
        'scanner: {
            let scanner = scanners.next();
            assert_eq!(true, scanner.is_some());

            let scanner = scanner.unwrap();
            assert_eq!(StatementScannerType::Statement, scanner.scanner_type());
            assert_eq!(9..18, scanner.scan_range());

            let mut scanner_view = scanner.as_view(..);
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::SELECT, offset: 9, len: 6, value: Some("SELECT".into()) },
                    trailing_trivia: Some(vec![ScanEvent{ kind: syntax_kind::SPACE, offset: 15, len: 1, value: Some(" ".into()) }]),
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 16, len: 1, value: Some("1".into()) },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::SEMI, offset: 17, len: 1, value: Some(";".into()) },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            break 'scanner;
        }
        'scanner: {
            let scanner = scanners.next();
            assert_eq!(true, scanner.is_some());

            let scanner = scanner.unwrap();
            assert_eq!(StatementScannerType::Eof, scanner.scanner_type());
            assert_eq!(18..18, scanner.scan_range());

            let mut scanner_view = scanner.as_view(..);
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::EOF, offset: 18, len: 0, value: None },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            break 'scanner;
        }
        'scanner: {
            let scanner = scanners.next();
            assert_eq!(false, scanner.is_some());
            break 'scanner;
        }

        Ok(())
    }

    #[test]
    fn test_remove_statement() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 2;SELECT 3;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 9,
            old_char_len: 18,
            new_char_len: 0,
            text: "".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let mut scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;

        'scanner: {
            let scanner = scanners.next();
            assert_eq!(true, scanner.is_some());

            let scanner = scanner.unwrap();
            assert_eq!(StatementScannerType::Statement, scanner.scanner_type());
            assert_eq!(0..9, scanner.scan_range());

            let mut scanner_view = scanner.as_view(..);
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::SELECT, offset: 0, len: 6, value: Some("SELECT".into()) },
                    trailing_trivia: Some(vec![ScanEvent{ kind: syntax_kind::SPACE, offset: 6, len: 1, value: Some(" ".into()) }]),
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 7, len: 1, value: Some("1".into()) },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::SEMI, offset: 8, len: 1, value: Some(";".into()) },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            break 'scanner;
        }
        'scanner: {
            let scanner = scanners.next();
            assert_eq!(true, scanner.is_some());

            let scanner = scanner.unwrap();
            assert_eq!(StatementScannerType::Eof, scanner.scanner_type());
            assert_eq!(9..9, scanner.scan_range());

            let mut scanner_view = scanner.as_view(..);
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::EOF, offset: 9, len: 0, value: None },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            break 'scanner;
        }
        'scanner: {
            let scanner = scanners.next();
            assert_eq!(false, scanner.is_some());
            break 'scanner;
        }

        Ok(())
    }

    #[test]
    fn test_append_word() -> Result<(), anyhow::Error> {
        let source = "SELECT 1";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 8,
            old_char_len: 0,
            new_char_len: 1,
            text: "2".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let mut scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;

        'scanner: {
            let scanner = scanners.next();
            assert_eq!(true, scanner.is_some());

            let scanner = scanner.unwrap();
            assert_eq!(StatementScannerType::Statement, scanner.scanner_type());
            assert_eq!(0..9, scanner.scan_range());

            let mut scanner_view = scanner.as_view(..);
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::SELECT, offset: 0, len: 6, value: Some("SELECT".into()) },
                    trailing_trivia: Some(vec![ScanEvent{ kind: syntax_kind::SPACE, offset: 6, len: 1, value: Some(" ".into()) }]),
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 7, len: 2, value: Some("12".into()) },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            break 'scanner;
        }
        'scanner: {
            let scanner = scanners.next();
            assert_eq!(true, scanner.is_some());

            let scanner = scanner.unwrap();
            assert_eq!(StatementScannerType::Eof, scanner.scanner_type());
            assert_eq!(9..9, scanner.scan_range());

            let mut scanner_view = scanner.as_view(..);
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::EOF, offset: 9, len: 0, value: None },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            break 'scanner;
        }
        'scanner: {
            let scanner = scanners.next();
            assert_eq!(false, scanner.is_some());
            break 'scanner;
        }

        Ok(())
    }

    #[test]
    fn test_prepend_word() -> Result<(), anyhow::Error> {
        let source = "ELECT 1";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 1,
            text: "S".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let mut scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;

        'scanner: {
            let scanner = scanners.next();
            assert_eq!(true, scanner.is_some());

            let scanner = scanner.unwrap();
            assert_eq!(StatementScannerType::Statement, scanner.scanner_type());
            assert_eq!(0..8, scanner.scan_range());

            let mut scanner_view = scanner.as_view(..);
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::SELECT, offset: 0, len: 6, value: Some("SELECT".into()) },
                    trailing_trivia: Some(vec![ScanEvent{ kind: syntax_kind::SPACE, offset: 6, len: 1, value: Some(" ".into()) }]),
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 7, len: 1, value: Some("1".into()) },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            break 'scanner;
        }
        'scanner: {
            let scanner = scanners.next();
            assert_eq!(true, scanner.is_some());

            let scanner = scanner.unwrap();
            assert_eq!(StatementScannerType::Eof, scanner.scanner_type());
            assert_eq!(8..8, scanner.scan_range());

            let mut scanner_view = scanner.as_view(..);
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::EOF, offset: 8, len: 0, value: None },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            break 'scanner;
        }
        'scanner: {
            let scanner = scanners.next();
            assert_eq!(false, scanner.is_some());
            break 'scanner;
        }

        Ok(())
    }

    #[test]
    fn test_append_comment() -> Result<(), anyhow::Error> {
        let source = "";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 13,
            text: "/* comment */".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let mut scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;

        'scanner: {
            let scanner = scanners.next();
            assert_eq!(true, scanner.is_some());

            let scanner = scanner.unwrap();
            assert_eq!(StatementScannerType::Eof, scanner.scanner_type());
            assert_eq!(0..13, scanner.scan_range());

            let mut scanner_view = scanner.as_view(..);
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: Some(vec![
                        ScanEvent{ kind: syntax_kind::COMMENT, offset: 0, len: 13, value: Some("/* comment */".into()) }
                    ]),
                    main: ScanEvent{ kind: syntax_kind::EOF, offset: 13, len: 0, value: None },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            break 'scanner;
        }
        'scanner: {
            let scanner = scanners.next();
            assert_eq!(false, scanner.is_some());
            break 'scanner;
        }
        Ok(())
    }

    #[test]
    fn test_append_new_line() -> Result<(), anyhow::Error> {
        let source = "\n";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 1,
            old_char_len: 0,
            new_char_len: 1,
            text: "\n".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let mut scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;

        'scanner: {
            let scanner = scanners.next();
            assert_eq!(true, scanner.is_some());

            let scanner = scanner.unwrap();
            assert_eq!(StatementScannerType::Eof, scanner.scanner_type());
            assert_eq!(0..2, scanner.scan_range());

            let mut scanner_view = scanner.as_view(..);
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: Some(vec![
                        ScanEvent{ kind: syntax_kind::SPACE, offset: 0, len: 2, value: Some("\n\n".into()) }
                    ]),
                    main: ScanEvent{ kind: syntax_kind::EOF, offset: 2, len: 0, value: None },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            break 'scanner;
        }
        'scanner: {
            let scanner = scanners.next();
            assert_eq!(false, scanner.is_some());
            break 'scanner;
        }
        Ok(())
    }

    #[test]
    fn test_insert_within_statement() -> Result<(), anyhow::Error> {
        let source = "SELCT 1;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 3,
            old_char_len: 0,
            new_char_len: 1,
            text: "E".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let mut scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;

        'scanner: {
            let scanner = scanners.next();
            assert_eq!(true, scanner.is_some());

            let scanner = scanner.unwrap();
            assert_eq!(StatementScannerType::Statement, scanner.scanner_type());
            assert_eq!(0..9, scanner.scan_range());

            let mut scanner_view = scanner.as_view(..);
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::SELECT, offset: 0, len: 6, value: Some("SELECT".into()) },
                    trailing_trivia: Some(vec![ScanEvent{ kind: syntax_kind::SPACE, offset: 6, len: 1, value: Some(" ".into()) }]),
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);

                break 'lookahead;
            }
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 7, len: 1, value: Some("1".into()) },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);

                break 'lookahead;
            }
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::SEMI, offset: 8, len: 1, value: Some(";".into()) },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);

                break 'lookahead;
            }
            break 'scanner;
        }
        'scanner: {
            let scanner = scanners.next();
            assert_eq!(true, scanner.is_some());

            let scanner = scanner.unwrap();
            assert_eq!(StatementScannerType::Eof, scanner.scanner_type());
            assert_eq!(9..9, scanner.scan_range());

            let mut scanner_view = scanner.as_view(..);
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::EOF, offset: 9, len: 0, value: None },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            break 'scanner;
        }
        'scanner: {
            let scanner = scanners.next();
            assert_eq!(false, scanner.is_some());
            break 'scanner;
        }

        Ok(())
    }

    #[test]
    fn test_replace_within_statement() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 2;SELECT 3;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 8,
            old_char_len: 17,
            new_char_len: 1,
            text: "2".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let mut scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;

        'scanner: {
            let scanner = scanners.next();
            assert_eq!(true, scanner.is_some());

            let scanner = scanner.unwrap();
            assert_eq!(StatementScannerType::Statement, scanner.scanner_type());
            assert_eq!(0..11, scanner.scan_range());

            let mut scanner_view = scanner.as_view(..);
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::SELECT, offset: 0, len: 6, value: Some("SELECT".into()) },
                    trailing_trivia: Some(vec![ScanEvent{ kind: syntax_kind::SPACE, offset: 6, len: 1, value: Some(" ".into()) }]),
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 7, len: 3, value: Some("123".into()) },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::SEMI, offset: 10, len: 1, value: Some(";".into()) },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            break 'scanner;
        }
        'scanner: {
            let scanner = scanners.next();
            assert_eq!(true, scanner.is_some());

            let scanner = scanner.unwrap();
            assert_eq!(StatementScannerType::Eof, scanner.scanner_type());
            assert_eq!(11..11, scanner.scan_range());

            let mut scanner_view = scanner.as_view(..);
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::EOF, offset: 11, len: 0, value: None },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            break 'scanner;
        }
        'scanner: {
            let scanner = scanners.next();
            assert_eq!(false, scanner.is_some());
            break 'scanner;
        }

        Ok(())
    }

    #[test]
    fn test_replace_with_trivia() -> Result<(), anyhow::Error> {
        let source = "/* 日本語コメント */SELECT 42 AS a;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 23,
            old_char_len: 2,
            new_char_len: 14,
            text: "/* ASを取り除いた */".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let mut scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;

        'scanner: {
            let scanner = scanners.next();
            assert_eq!(true, scanner.is_some());

            let scanner = scanner.unwrap();
            assert_eq!(StatementScannerType::Statement, scanner.scanner_type());
            assert_eq!(0..66, scanner.scan_range());

            let mut scanner_view = scanner.as_view(..);
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: Some(vec![ScanEvent{ kind: syntax_kind::COMMENT, offset: 0, len: 27, value: Some("/* 日本語コメント */".into()) }]),
                    main: ScanEvent{ kind: syntax_kind::SELECT, offset: 27, len: 6, value: Some("SELECT".into()) },
                    trailing_trivia: Some(vec![ScanEvent{ kind: syntax_kind::SPACE, offset: 33, len: 1, value: Some(" ".into()) }]),
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 34, len: 2, value: Some("42".into()) },
                    trailing_trivia: Some(vec![ScanEvent{ kind: syntax_kind::SPACE, offset: 36, len: 1, value: Some(" ".into()) }]),
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: Some(vec![
                        ScanEvent{ kind: syntax_kind::COMMENT, offset: 37, len: 26, value: Some("/* ASを取り除いた */".into()) },
                        ScanEvent{ kind: syntax_kind::SPACE, offset: 63, len: 1, value: Some(" ".into()) },
                    ]),
                    main: ScanEvent{ kind: syntax_kind::ID, offset: 64, len: 1, value: Some("a".into()) },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::SEMI, offset: 65, len: 1, value: Some(";".into()) },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            break 'scanner;
        }
        'scanner: {
            let scanner = scanners.next();
            assert_eq!(true, scanner.is_some());

            let scanner = scanner.unwrap();
            assert_eq!(StatementScannerType::Eof, scanner.scanner_type());
            assert_eq!(66..66, scanner.scan_range());

            let mut scanner_view = scanner.as_view(..);
            'lookahead: {
                let Some(lookahead) = scanner_view.shift() else { unreachable!() };
                let expect_lookahead = Token{
                    leading_trivia: None,
                    main: ScanEvent{ kind: syntax_kind::EOF, offset: 66, len: 0, value: None },
                    trailing_trivia: None,
                };
                assert_eq!(expect_lookahead.leading_trivia, lookahead.leading_trivia);
                assert_eq!(expect_lookahead.main, lookahead.main);
                assert_eq!(expect_lookahead.trailing_trivia, lookahead.trailing_trivia);
                break 'lookahead;
            }
            break 'scanner;
        }
        'scanner: {
            let scanner = scanners.next();
            assert_eq!(false, scanner.is_some());
            break 'scanner;
        }

        Ok(())
    }
}

mod edit_hint_eval_tests {
    use tolerant_parser_sdk::core::engine_core::scanner_engine::CaseSensitivity;
    use tolerant_parser_sdk::core::parser_core::{incremental::edit_hint::EditHint, ParseMode, RecoveryPenalty};
    use super::*;

    #[test]
    fn test_eval_edit_hint_for_append_statement() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 9,
            old_char_len: 0,
            new_char_len: 9,
            text: "SELECT 2;".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![None], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(9..18)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(1, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_append_statement_many() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 9,
            old_char_len: 0,
            new_char_len: 18,
            text: "SELECT 2;SELECT 3;".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![None, None], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(9..18), Some(18..27), ], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(1, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_append_token() -> Result<(), anyhow::Error> {
        let source = "SELECT 1";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 8,
            old_char_len: 0,
            new_char_len: 1,
            text: "6".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(0)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(0..9)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(0, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_append_semicolon() -> Result<(), anyhow::Error> {
        let source = "SELECT 1";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 8,
            old_char_len: 0,
            new_char_len: 1,
            text: ";".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(0)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(0..9)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(0, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_append_new_line() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 9,
            old_char_len: 0,
            new_char_len: 1,
            text: "\n".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(0)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(0..10)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(0, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_append_new_line_without_semicolon() -> Result<(), anyhow::Error> {
        let source = "SELECT 1";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 8,
            old_char_len: 0,
            new_char_len: 1,
            text: "\n".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(0)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(0..9)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(0, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_append_comment_without_semicolon() -> Result<(), anyhow::Error> {
        let source = "SELECT 1";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 8,
            old_char_len: 0,
            new_char_len: 13,
            text: "/* comment */".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(1)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(8..21)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(1, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_append_new_line_only() -> Result<(), anyhow::Error> {
        let source = "\n";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 1,
            old_char_len: 0,
            new_char_len: 1,
            text: "\n".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(0)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(0..2)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(0, result.replace_from);

        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_remove_from_new_line_only() -> Result<(), anyhow::Error> {
        let source = "\n\n";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 1,
            old_char_len: 1,
            new_char_len: 0,
            text: "".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(0)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(0..1)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(0, result.replace_from);

        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_append_line_comment_after_statement() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;\n-";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 11,
            old_char_len: 0,
            new_char_len: 1,
            text: "-".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(1), Some(2)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![None, Some(10..12)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(1, result.replace_from);

        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_append_line_comment_after_statement_2() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;\nSELECT 2;\n\n";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 20,
            old_char_len: 0,
            new_char_len: 1,
            text: "-".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(1), None], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(10..20), Some(20..22)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(1, result.replace_from);

        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_no_append() -> Result<(), anyhow::Error> {
        let source = "SELECT 1";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 8,
            old_char_len: 0,
            new_char_len: 0,
            text: "".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(Vec::<Option<usize>>::new(), result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(Vec::<Option<std::ops::Range<usize>>>::new(), result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(1, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_prepend_statement() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 9,
            text: "SELECT 0;".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![None], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(0..9)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(0, result.replace_from);
        Ok(())
   }

    #[test]
    fn test_eval_edit_hint_for_prepend_statement_many() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 19,
            text: "SELECT -1;SELECT 0;".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![None, None], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(0..10), Some(10..19)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        Ok(())
   }

    #[test]
    fn test_eval_edit_hint_for_prepend_trivia() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 1,
            text: "\n".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(0)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(0..10)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(0, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_prepend_token() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 22,
            text: "WITH v AS (SELECT 42) ".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(0)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(0..31)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(0, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_prepend_token_with_new_statements() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 33,
            text: "SELECT 'a';SELECT 'b';WITH v AS (SELECT 42) ".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![None, None, Some(0)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(0..11), Some(11..22), Some(22..53)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(0, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_no_prepend() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 0,
            text: "".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(Vec::<Option<usize>>::new(), result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(Vec::<Option<std::ops::Range<usize>>>::new(), result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(0, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_insert_statement() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 2;SELECT 3;SELECT 4;";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 18,
            old_char_len: 0,
            new_char_len: 22,
            text: "SELECT 'a';SELECT 'b';".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![None, None], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(18..29), Some(29..40)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(2, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_insert_by_appending_trivia() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 2;SELECT 3;SELECT 4;";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 18,
            old_char_len: 0,
            new_char_len: 1,
            text: "\n".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(1)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(9..19)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(1, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_insert_by_prepending_trivia() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 2;SELECT 3;SELECT 4;";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 18,
            old_char_len: 0,
            new_char_len: 16,
            text: "/* (comment) */ ".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(2)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(18..43)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(2, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_insert_by_prepending_token() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 2;SELECT 3;SELECT 4;";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 18,
            old_char_len: 0,
            new_char_len: 22,
            text: "WITH v AS (SELECT 42) ".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(2)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(18..49)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(2, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_no_insert() -> Result<(), anyhow::Error> {
        let source = "SELECT 4;SELECT 3;SELECT 2";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 9,
            old_char_len: 0,
            new_char_len: 0,
            text: "".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(Vec::<Option<usize>>::new(), result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(Vec::<Option<std::ops::Range<usize>>>::new(), result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(1, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_insert_within_statement() -> Result<(), anyhow::Error> {
        let source = "SELCT 1;";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 3,
            old_char_len: 0,
            new_char_len: 1,
            text: "E".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(0)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(0..9)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(0, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_update_single() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 2;SELECT 3;";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 16,
            old_char_len: 1,
            new_char_len: 2,
            text: "33".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(1)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(9..19)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(1, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_update_many() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 2;SELECT 3;SELECT 4;SELECT 5;";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 16,
            old_char_len: 18,
            new_char_len: 21,
            text: "42;SELECT 43;SELECT 4".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(1), Some(2), Some(3)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(9..19), Some(19..29), Some(29..39)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(1, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_update_by_splitting() -> Result<(), anyhow::Error> {
        let source = "SELECT 42;";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 8,
            old_char_len: 0,
            new_char_len: 17,
            text: ";SELECT 3;SELECT ".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![None, None, Some(0)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(0..9), Some(9..18), Some(18..27)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(0, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_update_by_merging() -> Result<(), anyhow::Error> {
        let source = "SELECT 4;SELECT 3;SELECT 2;";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 8,
            old_char_len: 17,
            new_char_len: 0,
            text: "".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(0), Some(1), Some(2)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![None, None, Some(0..10)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(0, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_update_by_splitting_with_semicolon() -> Result<(), anyhow::Error> {
        let source = "SELECT 1 AS x;";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 9,
            old_char_len: 2,
            new_char_len: 17,
            text: "AS y; SELECT 2 AS".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![None, Some(0)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(0..15), Some(15..29)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(0, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_update_by_removing_stateent() -> Result<(), anyhow::Error> {
        let source = "SELECT 4;\nSELECT 3;\nSELECT 2;\nSELECT 1;\n";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 10,
            old_char_len: 20,
            new_char_len: 0,
            text: "".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(1), Some(2)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![None, None], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(1, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_update_by_removing_stateent_except_trivia() -> Result<(), anyhow::Error> {
        let source = "SELECT 4;\nSELECT 3;\nSELECT 2;\nSELECT 1;\n";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 10,
            old_char_len: 19,
            new_char_len: 0,
            text: "".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(0), Some(1), Some(2)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![None, None, Some(0..11)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(0, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_update_by_removing_stateent_except_comment() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;\n/* comment */SELECT 2;\nSELECT 3;\nSELECT 4;\n";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 23,
            old_char_len: 30,
            new_char_len: 0,
            text: "".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(1), Some(2), Some(3), Some(4)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![None, None, None, Some(10..23)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(1, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_update_by_removing_all() -> Result<(), anyhow::Error> {
        let source = "SELECT 4;SELECT 3;SELECT 2";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 26,
            new_char_len: 0,
            text: "".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(0), Some(1), Some(2)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![None, None, None], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(0, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_update_by_removing() -> Result<(), anyhow::Error> {
        let source = "SELECT 4;SELECT 3;SELECT 2";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 1,
            new_char_len: 0,
            text: "".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![Some(0)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(0..8)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(0, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_remove_line_comment_after_statement() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;\n--#1\n--#2";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 11,
            old_char_len: 3,
            new_char_len: 0,
            text: "".into(),
        };

        let hint = EditHint::new(&tree, scope.old_char_range());
        let stmt_scanners = hint.reconcile_lookaheads(scope.old_char_range(), &scope.text, engine.scanning_rules, engine.parsing_rules, config.case_sensitive)?;
        let result = hint.eval_hint(stmt_scanners);

        assert_eq!(vec![None, Some(1)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(10..12), Some(12..16)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(1, result.replace_from);

        Ok(())
    }

}
