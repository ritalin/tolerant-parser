use engine_core::SyntaxKind;
use parser_core::{incremental::EditScope, syntax_tree::SyntaxNode, Parser};
use sqlite_engine::syntax_kind;

mod expand_region_tests {
    use parser_core::syntax_tree::MetadataAccess;

    use super::*;

    fn extend_to_neighbors(scope: std::ops::Range<usize>, root: Option<&SyntaxNode>, except_kind: SyntaxKind) -> std::ops::Range<usize> {
        let Some(root) = root else { return scope; };

        let adjusted_range = parser_core::incremental::support::adjust_edit_range(&scope, &root.metadata_key().byte_range());

        let gardener = parser_core::incremental::support::TreeGardener::new(root);
        let anscestor = gardener.common_anscestor(
            gardener.pick_token((adjusted_range.start as u32).into()),
            gardener.pick_token((adjusted_range.end as u32).into()),
            except_kind
        );
        anscestor.unwrap().node.text_range().into()
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

    #[test]
    fn test_find_edit_statements() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 2;SELECT 3;SELECT 4;SELECT 5;";
        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine);
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 10,
            old_char_len: 20,
            new_char_len: 22,
        };

        let indexes = parser_core::incremental::find_edit_statements(&tree, &scope)
            .map(|node| node.into_raw().index())
            .collect::<Vec<_>>()
        ;
        assert_eq!(vec![1,2,3], indexes);
        Ok(())
    }
}

mod incremental_support_tests {
    use parser_core::incremental::support;

    use super::*;

    #[test]
    fn text_prev_token() -> Result<(), anyhow::Error> {
        let source = "SELECT 101 AS x FROM foo u;";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let root_node = tree.root();
        let gardener = support::TreeGardener::new(&root_node);
        let token = gardener.pick_token(11.into());
        assert_eq!(Some(syntax_kind::AS.id), token.as_ref().map(|x| x.token.kind()));
        assert_eq!(true, token.as_ref().unwrap().clone().token.text_range().contains(11.into()));

        let neighbor = token.as_ref().map(|x| x.clone().into_prev(&gardener.node, syntax_kind::SEMI));
        assert_eq!(Some(syntax_kind::SPACE), neighbor.as_ref().map(|x| engine.parsing_rules.from_kind_id(x.token.kind())));
        assert_eq!(true, neighbor.as_ref().unwrap().clone().token.text_range().contains(10.into()));

        Ok(())
    }

    #[test]
    fn text_prev_token_on_trivia() -> Result<(), anyhow::Error> {
        let source = "SELECT 101 AS x FROM foo u;";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let root_node = tree.root();
        let gardener = support::TreeGardener::new(&root_node);
        let token = gardener.pick_token(10.into());
        assert_eq!(Some(syntax_kind::SPACE.id), token.as_ref().map(|x| x.token.kind()));
        assert_eq!(true, token.as_ref().unwrap().clone().token.text_range().contains(10.into()));

        let neighbor = token.as_ref().map(|x| x.clone().into_prev(&gardener.node, syntax_kind::SEMI));
        assert_eq!(Some(syntax_kind::SPACE), neighbor.as_ref().map(|x| engine.parsing_rules.from_kind_id(x.token.kind())));
        assert_eq!(true, neighbor.as_ref().unwrap().clone().token.text_range().contains(6.into()));

        Ok(())
    }

    #[test]
    fn text_prev_token_on_semicollon() -> Result<(), anyhow::Error> {
        let source = "SELECT 101 AS x FROM foo u;";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let root_node = tree.root();
        let gardener = support::TreeGardener::new(&root_node);
        let token = gardener.pick_token(26.into());
        assert_eq!(Some(syntax_kind::SEMI.id), token.as_ref().map(|x| x.token.kind()));
        assert_eq!(true, token.as_ref().unwrap().clone().token.text_range().contains(26.into()));

        let neighbor = token.as_ref().map(|x| x.clone().into_prev(&gardener.node, syntax_kind::SEMI));
        assert_eq!(Some(syntax_kind::ID), neighbor.as_ref().map(|x| engine.parsing_rules.from_kind_id(x.token.kind())));
        assert_eq!(true, neighbor.as_ref().unwrap().clone().token.text_range().contains(25.into()));

        Ok(())
    }

    #[test]
    fn text_next_token() -> Result<(), anyhow::Error> {
        let source = "SELECT 101 AS x FROM foo u;";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let root_node = tree.root();
        let gardener = support::TreeGardener::new(&root_node);
        let token = gardener.pick_token(11.into());
        assert_eq!(Some(syntax_kind::AS.id), token.as_ref().map(|x| x.token.kind()));
        assert_eq!(true, token.as_ref().unwrap().clone().token.text_range().contains(11.into()));

        let neighbor = token.as_ref().map(|x| x.clone().into_next(&gardener.node, syntax_kind::SEMI));
        assert_eq!(Some(syntax_kind::ID), neighbor.as_ref().map(|x| engine.parsing_rules.from_kind_id(x.token.kind())));
        assert_eq!(true, neighbor.as_ref().unwrap().clone().token.text_range().contains(14.into()));

        Ok(())
    }

    #[test]
    fn text_next_token_on_trivia() -> Result<(), anyhow::Error> {
        let source = "SELECT /*VALUE*/101 AS x FROM foo u;";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let root_node = tree.root();
        let gardener = support::TreeGardener::new(&root_node);
        let token = gardener.pick_token(11.into());
        assert_eq!(Some(syntax_kind::COMMENT.id), token.as_ref().map(|x| x.token.kind()));
        assert_eq!(true, token.as_ref().unwrap().clone().token.text_range().contains(11.into()));

        let neighbor = token.as_ref().map(|x| x.clone().into_next(&gardener.node, syntax_kind::SEMI));
        assert_eq!(Some(syntax_kind::AS), neighbor.as_ref().map(|x| engine.parsing_rules.from_kind_id(x.token.kind())));
        assert_eq!(true, neighbor.as_ref().unwrap().clone().token.text_range().contains(20.into()));

        Ok(())
    }

    #[test]
    fn text_next_token_on_semicollon() -> Result<(), anyhow::Error> {
        let source = "SELECT 101 AS x FROM foo u; SELECT 1;";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let root_node = tree.root();
        let gardener = support::TreeGardener::new(&root_node);
        let token = gardener.pick_token(26.into());
        assert_eq!(Some(syntax_kind::SEMI.id), token.as_ref().map(|x| x.token.kind()));
        assert_eq!(true, token.as_ref().unwrap().clone().token.text_range().contains(26.into()));

        let neighbor = token.as_ref().map(|x| x.clone().into_next(&gardener.node, syntax_kind::SEMI));
        assert_eq!(Some(syntax_kind::SEMI), neighbor.as_ref().map(|x| engine.parsing_rules.from_kind_id(x.token.kind())));
        assert_eq!(true, neighbor.as_ref().unwrap().clone().token.text_range().contains(26.into()));

        Ok(())
    }

    #[test]
    fn text_find_least_common_anscestor() -> Result<(), anyhow::Error> {
        let source = "SELECT 101 AS x FROM foo u;";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let stmt_node = tree.root().nth_child(0).and_then(|el| el.to_node()).unwrap();
        let gardener = support::TreeGardener::new(&stmt_node);
        
        let lhs = gardener.pick_token(11.into());
        let rhs = gardener.pick_token(12.into());

        let anscestor = gardener.common_anscestor(lhs, rhs, syntax_kind::SEMI);
        assert_eq!(Some(syntax_kind::selcollist), anscestor.map(|x| engine.parsing_rules.from_kind_id(x.node.kind())));
        Ok(())
    }
}

mod parser_tests {
    use parser_core::{syntax_tree::{ApplyBatch, NodeOperation, SyntaxTokenItem}, ParseMode, ParserConfig, RecoveryPenalty};

    use crate::test_support::{self, ExpectNode};

    use super::*;

    fn rebuild_source(token: Option<SyntaxTokenItem>) -> String {
        let mut tokens = vec![];
        let mut next_token = token;

        while let Some(x) = next_token {
            tokens.push(x.value().to_string());
            next_token = x.next_sibling().clone();
        }

        tokens.join("")
    }

    #[test]
    fn test_parse_single_statement_with_inserting() -> Result<(), anyhow::Error> {
        let source = "SELECT 42 x FROM foo u;";
        let new_source = "SELECT 42 AS x FROM foo u;";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let rebuilded_source = rebuild_source(tree.root().token_at_utf16_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_char_offset: 10,
            old_char_len: 0,
            new_char_len: 3,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_single_statement_with_inserting.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_single_statement_with_deleting() -> Result<(), anyhow::Error> {
        let source =     "SELECT 42 AS x FROM foo u;";
        let new_source = "SELECT 42 x FROM foo u;";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let rebuilded_source = rebuild_source(tree.root().token_at_utf16_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_char_offset: 10,
            old_char_len: 3,
            new_char_len: 0,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_single_statement_with_deleting.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_single_with_cross_over_2_statements() -> Result<(), anyhow::Error> {
        let source = "SELECT '101'; SELECT 42 x FROM foo u;";
        let new_source = "SELECT 42; SELECT p, 42 x FROM foo u;";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let rebuilded_source = rebuild_source(tree.root().token_at_utf16_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_char_offset: 7,
            old_char_len: 14,
            new_char_len: 14,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_single_with_cross_over_2_statements.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_append_statment() -> Result<(), anyhow::Error> {
        let source = "SELECT '101';";
        let new_source = "SELECT '101'; SELECT 42 x FROM foo u;";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let rebuilded_source = rebuild_source(tree.root().token_at_utf16_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_char_offset: 13,
            old_char_len: 0,
            new_char_len: 24,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_append_statment.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_prepend_statment() -> Result<(), anyhow::Error> {
        let source = "SELECT '101';";
        let new_source = " SELECT 42 x FROM foo u;SELECT '101';";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let rebuilded_source = rebuild_source(tree.root().token_at_utf16_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 24,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_prepend_statment.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_with_maltibyte_char() -> Result<(), anyhow::Error> {
        let source = "/* 日本語コメント */SELECT 42 AS a;";
        let new_source = "/* 日本語コメント */SELECT 42 /* ASを取り除いた */ a;";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let rebuilded_source = rebuild_source(tree.root().token_at_utf16_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_char_offset: 23,
            old_char_len: 2,
            new_char_len: 14,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_with_maltibyte_char.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_remove_all() -> Result<(), anyhow::Error> {
        let source = "SELECT 42 x FROM foo u;";
        let new_source = "";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let rebuilded_source = rebuild_source(tree.root().token_at_utf16_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 23,
            new_char_len: 0,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_remove_all.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_insert_from_empty() -> Result<(), anyhow::Error> {
        let source = "";
        let new_source = "SELECT 42 AS x FROM foo u;";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let rebuilded_source = rebuild_source(tree.root().token_at_utf16_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 26,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_insert_from_empty.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_split_statement_on_inserting_semicolon() -> Result<(), anyhow::Error> {
        let source = "SELECT 1 AS x;";
        let new_source = "SELECT 1 AS y; SELECT 2 AS x;";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let rebuilded_source = rebuild_source(tree.root().token_at_utf16_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_char_offset: 9,
            old_char_len: 2,
            new_char_len: 17,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_split_statement_on_inserting_semicolon.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_broken_keyword_by_inserting() -> Result<(), anyhow::Error> {
        let source = "S";
        let new_source = "SE";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 1,
            old_char_len: 0,
            new_char_len: 1,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_broken_keyword_by_inserting.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_broken_keyword_by_removing_first() -> Result<(), anyhow::Error> {
        let source = "SELECT";
        let new_source = "ELECT";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 1,
            new_char_len: 0,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_broken_keyword_by_removing_first.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_broken_keyword_by_inserting_first() -> Result<(), anyhow::Error> {
        let source = "ELECT";
        let new_source = "SELECT";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 1,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_broken_keyword_by_inserting_first.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_statement_by_appending_semicolon() -> Result<(), anyhow::Error> {
        let source = "SELECT 42";
        let new_source = "SELECT 42;";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 9,
            old_char_len: 0,
            new_char_len: 1,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_statement_by_appending_semicolon.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_statement_by_dropping_semicolon() -> Result<(), anyhow::Error> {
        let source = "SELECT 42;";
        let new_source = "SELECT 42";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 9,
            old_char_len: 1,
            new_char_len: 0,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_statement_by_dropping_semicolon.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_statement_by_filling_value() -> Result<(), anyhow::Error> {
        let source = "SELECT ";
        let new_source = "SELECT 42;";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 7,
            old_char_len: 0,
            new_char_len: 2,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_statement_by_filling_value.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    // FIXME: fn test_parse_keyword_only_with_semicolon() // SELECT -> SELECT;
}