
use tolerant_parser_sdk::core::engine_core::{SyntaxKind, scanner_engine::CaseSensitivity};
use tolerant_parser_sdk::core::parser_core::{incremental::EditScope, syntax_tree::SyntaxNode, Parser, ParserConfig, ParseMode, RecoveryPenalty};
use sqlite_engine::syntax_kind;

mod expand_region_tests {
    use tolerant_parser_sdk::core::parser_core::{self, syntax_tree::MetadataAccess};

    use super::*;

    fn extend_to_neighbors(scope: std::ops::Range<usize>, root: Option<&SyntaxNode>, except_kind: SyntaxKind) -> std::ops::Range<usize> {
        let Some(root) = root else { return scope; };

        let adjusted_range = parser_core::incremental::support::adjust_edit_range(&scope, &root.metadata_key().byte_range());

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
            new_char_len: 34,
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
            new_char_len: 23,
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
            new_char_len: 22,
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
            new_char_len: 22,
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
            new_char_len: 22,
        };

        let hint = EditHint::new(&tree, scope.old_char_range());

        assert_eq!(vec![1, 0], hint.precedings.into_iter().flatten().map(|node| node.into_raw().index()).collect::<Vec<_>>());
        assert_eq!(Vec::<usize>::new(), hint.statements.into_iter().map(|stmt| stmt.into_raw().index()).collect::<Vec<_>>());
        assert_eq!(vec![2, 3], hint.followings.into_iter().flatten().map(|node| node.into_raw().index()).collect::<Vec<_>>());
        Ok(())
    }
}

mod edit_hint_eval_tests {
    use tolerant_parser_sdk::core::engine_core::scanner_engine::CaseSensitivity;
    use tolerant_parser_sdk::core::parser_core::{incremental::edit_hint::EditHint, ParseMode, RecoveryPenalty};
    use tolerant_parser_sdk::core::scanner_core::Scanner;
    use super::*;

    #[test]
    fn test_eval_edit_hint_for_append_statement() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let new_source = "SELECT 1;SELECT 2;";
        let scope = EditScope{
            start_char_offset: 9,
            old_char_len: 0,
            new_char_len: 9,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 1;SELECT 2;SELECT 3;";
        let scope = EditScope{
            start_char_offset: 9,
            old_char_len: 0,
            new_char_len: 18,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 16";
        let scope = EditScope{
            start_char_offset: 8,
            old_char_len: 0,
            new_char_len: 1,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 1;";
        let scope = EditScope{
            start_char_offset: 8,
            old_char_len: 0,
            new_char_len: 1,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 1;\n";
        let scope = EditScope{
            start_char_offset: 9,
            old_char_len: 0,
            new_char_len: 1,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 1\n";
        let scope = EditScope{
            start_char_offset: 8,
            old_char_len: 0,
            new_char_len: 1,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 1/* comment */";
        let scope = EditScope{
            start_char_offset: 8,
            old_char_len: 0,
            new_char_len: 13,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "\n\n";
        let scope = EditScope{
            start_char_offset: 1,
            old_char_len: 0,
            new_char_len: 1,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "\n";
        let scope = EditScope{
            start_char_offset: 1,
            old_char_len: 1,
            new_char_len: 0,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 1;\n--";
        let scope = EditScope{
            start_char_offset: 11,
            old_char_len: 0,
            new_char_len: 1,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 1;\nSELECT 2;\n-\n";
        let scope = EditScope{
            start_char_offset: 20,
            old_char_len: 0,
            new_char_len: 1,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 1";
        let scope = EditScope{
            start_char_offset: 8,
            old_char_len: 0,
            new_char_len: 0,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 0;SELECT 1;";
        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 9,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT -1;SELECT 0;SELECT 1;";
        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 19,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "\nSELECT 1;";
        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 1,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "WITH v AS (SELECT 42) SELECT 1;";
        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 22,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 'a';SELECT 'b';WITH v AS (SELECT 42) SELECT 1;";
        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 33,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 1;";
        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 0,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 1;SELECT 2;SELECT 'a';SELECT 'b';SELECT 3;SELECT 4;";
        let scope = EditScope{
            start_char_offset: 18,
            old_char_len: 0,
            new_char_len: 22,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 1;SELECT 2;\nSELECT 3;SELECT 4;";
        let scope = EditScope{
            start_char_offset: 18,
            old_char_len: 0,
            new_char_len: 1,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 1;SELECT 2;/* (comment) */ SELECT 3;SELECT 4;";
        let scope = EditScope{
            start_char_offset: 18,
            old_char_len: 0,
            new_char_len: 16,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 1;SELECT 2;WITH v AS (SELECT 42) SELECT 3;SELECT 4;";
        let scope = EditScope{
            start_char_offset: 18,
            old_char_len: 0,
            new_char_len: 22,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 4;SELECT 3;SELECT 2";
        let scope = EditScope{
            start_char_offset: 9,
            old_char_len: 0,
            new_char_len: 0,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

        assert_eq!(Vec::<Option<usize>>::new(), result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(Vec::<Option<std::ops::Range<usize>>>::new(), result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(1, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_update_single() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 2;SELECT 3;";
        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let new_source = "SELECT 1;SELECT 33;SELECT 3;";
        let scope = EditScope{
            start_char_offset: 16,
            old_char_len: 1,
            new_char_len: 2,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 1;SELECT 42;SELECT 43;SELECT 44;SELECT 5;";
        let scope = EditScope{
            start_char_offset: 16,
            old_char_len: 18,
            new_char_len: 21,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 4;SELECT 3;SELECT 2;";
        let scope = EditScope{
            start_char_offset: 8,
            old_char_len: 0,
            new_char_len: 17,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 42;";
        let scope = EditScope{
            start_char_offset: 8,
            old_char_len: 17,
            new_char_len: 0,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 1 AS y; SELECT 2 AS x;";
        let scope = EditScope{
            start_char_offset: 9,
            old_char_len: 2,
            new_char_len: 17,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 4;\nSELECT 1;\n";
        let scope = EditScope{
            start_char_offset: 10,
            old_char_len: 20,
            new_char_len: 0,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 4;\n\nSELECT 1;\n";
        let scope = EditScope{
            start_char_offset: 10,
            old_char_len: 19,
            new_char_len: 0,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 1;\n/* comment */";
        let scope = EditScope{
            start_char_offset: 23,
            old_char_len: 30,
            new_char_len: 0,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "";
        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 26,
            new_char_len: 0,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "ELECT 4;SELECT 3;SELECT 2";
        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 1,
            new_char_len: 0,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

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

        let new_source = "SELECT 1;\n-\n--#2";
        let scope = EditScope{
            start_char_offset: 11,
            old_char_len: 3,
            new_char_len: 0,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();
        let full_emit_region = engine.parsing_rules.full_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range());
        let scanner = Scanner::create_without_scan(new_source, hint.scan_from(), engine.scanning_rules, config.case_sensitive)?;
        let stmt_scanners = scanner.statement_scanners(emit_region.to_symbol, full_emit_region.to_symbol).collect::<Vec<_>>();
        let result = hint.eval_hint(stmt_scanners, scope.new_char_range());

        assert_eq!(vec![None, Some(1)], result.events.iter().map(|slot| slot.index()).collect::<Vec<_>>());
        assert_eq!(vec![Some(10..12), Some(12..16)], result.events.iter().map(|slot| slot.scanner().map(|scanner| scanner.scan_range())).collect::<Vec<_>>());
        assert_eq!(1, result.replace_from);

        Ok(())
    }

}
