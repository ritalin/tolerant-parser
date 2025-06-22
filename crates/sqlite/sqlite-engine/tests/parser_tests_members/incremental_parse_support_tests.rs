
use engine_core::SyntaxKind;
use parser_core::{incremental::EditScope, syntax_tree::SyntaxNode, Parser};
use sqlite_engine::syntax_kind;

mod expand_region_tests {
    use parser_core::syntax_tree::MetadataAccess;

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
        let parser = Parser::new(engine);
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
        let parser = Parser::new(engine);
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
        let parser = Parser::new(engine);
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
        let parser = Parser::new(engine);
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
    use parser_core::incremental::support::EditHint;
    use super::*;

    #[test]
    fn test_init_hint_edit_for_update() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 2;SELECT 3;SELECT 4;SELECT 5;";
        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine);
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 10,
            old_char_len: 20,
            new_char_len: 22,
        };

        let hint = EditHint::new(&tree, scope.old_char_range(), tree.root().children().last().as_ref());
        let EditHint::Update { candidates, replace_from } = hint else { unreachable!() };

        assert_eq!(vec![1, 2, 3], candidates.into_iter().map(|stmt| stmt.into_raw().index()).collect::<Vec<_>>());
        assert_eq!(1, replace_from);
        Ok(())
    }

    #[test]
    fn test_init_hint_edit_for_update_without_statement() -> Result<(), anyhow::Error> {
        let source = "";
        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine);
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 22,
        };

        let hint = EditHint::new(&tree, scope.old_char_range(), tree.root().children().last().as_ref());
        let EditHint::Update { candidates, replace_from } = hint else { unreachable!() };

        assert_eq!(Vec::<SyntaxNode>::new(), candidates);
        assert_eq!(0, replace_from);
        Ok(())
    }

    #[test]
    fn test_init_hint_edit_for_append() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 2;";
        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine);
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 18,
            old_char_len: 0,
            new_char_len: 22,
        };

        let hint = EditHint::new(&tree, scope.old_char_range(), tree.root().children().last().as_ref());
        let EditHint::Append { candidate } = hint else { unreachable!() };

        assert_eq!(1, candidate.into_raw().index());
        Ok(())
    }

    #[test]
    fn test_init_hint_edit_for_prepend() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 2;";
        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine);
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 22,
        };

        let hint = EditHint::new(&tree, scope.old_char_range(), tree.root().children().last().as_ref());
        let EditHint::Prepend { candidate } = hint else { unreachable!() };

        assert_eq!(0, candidate.into_raw().index());
        Ok(())
    }

    #[test]
    fn test_init_hint_edit_for_insert() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 2;SELECT 3;";
        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine);
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 18,
            old_char_len: 0,
            new_char_len: 22,
        };

        let hint = EditHint::new(&tree, scope.old_char_range(), tree.root().children().last().as_ref());
        let EditHint::InsertBetween { prev, next } = hint else { unreachable!() };

        assert_eq!(1, prev.into_raw().index());
        assert_eq!(2, next.into_raw().index());
        Ok(())
    }
}

mod edit_hint_eval_tests {
    use parser_core::incremental::support::EditHint;
    use scanner_core::Scanner;
    use super::*;

    #[test]
    fn test_eval_edit_hint_for_append_statement() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;";
        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let new_source = "SELECT 1;SELECT 2;";
        let scope = EditScope{
            start_char_offset: 9,
            old_char_len: 0,
            new_char_len: 9,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range(), tree.root().children().last().as_ref());
        let scan_from = 0;
        let scanner = Scanner::create_without_scan(new_source, scan_from, engine.scanning_rules)?;
        let result = hint.eval_hint(&scanner.statement_scanners(emit_region.to_symbol).collect(), scope.new_char_range(), &emit_region);

        assert_eq!(Vec::<usize>::new(), result.statements.into_iter().map(|node| node.into_raw().index()).collect::<Vec<_>>());
        assert_eq!(1, result.skip_scanner);
        assert_eq!(1, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_append_token() -> Result<(), anyhow::Error> {
        let source = "SELECT 1";
        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let new_source = "SELECT 16";
        let scope = EditScope{
            start_char_offset: 8,
            old_char_len: 0,
            new_char_len: 1,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range(), tree.root().children().last().as_ref());
        let scan_from = 0;
        let scanner = Scanner::create_without_scan(new_source, scan_from, engine.scanning_rules)?;
        let result = hint.eval_hint(&scanner.statement_scanners(emit_region.to_symbol).collect(), scope.new_char_range(), &emit_region);

        assert_eq!(vec![0], result.statements.into_iter().map(|node| node.into_raw().index()).collect::<Vec<_>>());
        assert_eq!(0, result.skip_scanner);
        assert_eq!(0, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_append_semicolon() -> Result<(), anyhow::Error> {
        let source = "SELECT 1";
        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let new_source = "SELECT 1;";
        let scope = EditScope{
            start_char_offset: 8,
            old_char_len: 0,
            new_char_len: 1,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range(), tree.root().children().last().as_ref());
        let scan_from = 0;
        let scanner = Scanner::create_without_scan(new_source, scan_from, engine.scanning_rules)?;
        let result = hint.eval_hint(&scanner.statement_scanners(emit_region.to_symbol).collect(), scope.new_char_range(), &emit_region);

        assert_eq!(vec![0], result.statements.into_iter().map(|node| node.into_raw().index()).collect::<Vec<_>>());
        assert_eq!(0, result.skip_scanner);
        assert_eq!(0, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_append_new_line() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;";
        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let new_source = "SELECT 1;\n";
        let scope = EditScope{
            start_char_offset: 9,
            old_char_len: 0,
            new_char_len: 1,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range(), tree.root().children().last().as_ref());
        let scan_from = 0;
        let scanner = Scanner::create_without_scan(new_source, scan_from, engine.scanning_rules)?;
        let result = hint.eval_hint(&scanner.statement_scanners(emit_region.to_symbol).collect(), scope.new_char_range(), &emit_region);

        assert_eq!(vec![0], result.statements.into_iter().map(|node| node.into_raw().index()).collect::<Vec<_>>());
        assert_eq!(0, result.skip_scanner);
        assert_eq!(0, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint_for_append_new_line_without_semicolon() -> Result<(), anyhow::Error> {
        let source = "SELECT 1";
        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let new_source = "SELECT 1\n";
        let scope = EditScope{
            start_char_offset: 8,
            old_char_len: 0,
            new_char_len: 1,
        };
        let emit_region = engine.parsing_rules.statement_emit_config();

        let hint = EditHint::new(&tree, scope.old_char_range(), tree.root().children().last().as_ref());
        let scan_from = 0;
        let scanner = Scanner::create_without_scan(new_source, scan_from, engine.scanning_rules)?;
        let result = hint.eval_hint(&scanner.statement_scanners(emit_region.to_symbol).collect(), scope.new_char_range(), &emit_region);

        assert_eq!(vec![0], result.statements.into_iter().map(|node| node.into_raw().index()).collect::<Vec<_>>());
        assert_eq!(0, result.skip_scanner);
        assert_eq!(0, result.replace_from);
        Ok(())
    }

    #[test]
    fn test_eval_edit_hint() -> Result<(), anyhow::Error> {
        todo!()
    }
}
