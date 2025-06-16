use engine_core::SyntaxKind;
use parser_core::{incremental::EditScope, syntax_tree::SyntaxNode, Parser};
use sqlite_engine::syntax_kind;

mod expand_region_tests {
    use super::*;

    fn extend_to_neighbors(scope: &EditScope, root: Option<&SyntaxNode>, except_kind: SyntaxKind) -> EditScope {
        let Some(root) = root else { return scope.clone() };

        let (lowest_offset, highest_offset) = scope.adjust_range(scope.old_byte_len, &root.into_raw());

        let gardener = parser_core::incremental::support::TreeGardener{ node: root.into_raw() };
        let anscestor = gardener.common_anscestor(
            gardener.pick_token(lowest_offset.into()),
            gardener.pick_token(highest_offset.into()),
            except_kind
        );
        let node_range = anscestor.unwrap().text_range();

        EditScope{
            start_byte_offset: node_range.start().into(),
            old_byte_len: scope.old_byte_len,
            new_byte_len: node_range.len().into(),
        }
    }

    #[test]
    fn test_extend_to_neighbor_without_node() -> Result<(), anyhow::Error> {
        let scope = EditScope {
            start_byte_offset: 11,
            old_byte_len: 23,
            new_byte_len: 34,
        };

        let new_scope = extend_to_neighbors(&scope, None, syntax_kind::SEMI);
        assert_eq!(scope, new_scope);
        Ok(())
    }

    #[test]
    fn test_extend_to_neighbor_for_fitting_node() -> Result<(), anyhow::Error> {
        let source = "SELECT 101 AS x FROM foo u;";
        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine);
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_byte_offset: 0,
            old_byte_len: 27,
            new_byte_len: 27,
        };
        let new_scope = extend_to_neighbors(&scope, Some(&tree.root()), syntax_kind::SEMI);

        let expect_scope = EditScope{
            start_byte_offset: 0,
            old_byte_len: 27,
            new_byte_len: 27,
        };
        assert_eq!(expect_scope, new_scope);
        Ok(())
    }

    #[test]
    fn test_extend_to_neighbor_for_overall_node() -> Result<(), anyhow::Error> {
        let source = "SELECT 42;SELECT 101 AS x FROM foo u;SELECT a.b FROM bar;";
        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine);
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_byte_offset: 7,
            old_byte_len: 33,
            new_byte_len: 23,
        };
        let new_scope = extend_to_neighbors(
            &scope, 
            tree.root().nth_child(1).unwrap().to_node().as_ref(),
            syntax_kind::SEMI
        );

        let expect_scope = EditScope{
            start_byte_offset: 10,
            old_byte_len: 33,
            new_byte_len: 27,
        };
        assert_eq!(expect_scope, new_scope);
        Ok(())
    }

    #[test]
    fn test_extend_to_neighbor_for_inside_node() -> Result<(), anyhow::Error> {
        let source = "SELECT 42;SELECT 101 AS x FROM foo u;SELECT a.xyz AS v FROM bar;";
        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine);
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_byte_offset: 45,
            old_byte_len: 4,
            new_byte_len: 3,
        };
        let new_scope = extend_to_neighbors(
            &scope,
            tree.root().nth_child(2).unwrap().to_node().as_ref(),
            syntax_kind::SEMI
        );

        let expect_scope = EditScope{
            start_byte_offset: 44,
            old_byte_len: 4,
            new_byte_len: 11,
        };
        assert_eq!(expect_scope, new_scope);
        Ok(())
    }

    #[test]
    fn test_extend_to_neighbor_cross_over_2_nodes() -> Result<(), anyhow::Error> {
        let source = "SELECT 42;SELECT 101 AS x FROM foo u;SELECT a.xyz AS v FROM bar;";
        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine);
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_byte_offset: 32,
            old_byte_len: 10,
            new_byte_len: 13,
        };

        'left_hand: {
            let new_scope = extend_to_neighbors(
                &scope,
                tree.root().nth_child(1).unwrap().to_node().as_ref(),
                syntax_kind::SEMI
            );

            let expect_scope = EditScope{
                start_byte_offset: 10,
                old_byte_len: 10,
                new_byte_len: 27,
            };
            assert_eq!(expect_scope, new_scope);
            break 'left_hand;
        }
        'right_hand: {
            let new_scope = extend_to_neighbors(
                &scope,
                tree.root().nth_child(2).unwrap().to_node().as_ref(),
                syntax_kind::SEMI
            );

            let expect_scope = EditScope{
                start_byte_offset: 37,
                old_byte_len: 10,
                new_byte_len: 26,
            };
            assert_eq!(expect_scope, new_scope);
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
            start_byte_offset: 10,
            old_byte_len: 20,
            new_byte_len: 22,
        };

        let indexes = parser_core::incremental::find_edit_statements(&tree, &scope)
            .map(|node| node.index())
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

        let gardener = support::TreeGardener{ node: tree.root().into_raw() };
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

        let gardener = support::TreeGardener{ node: tree.root().into_raw() };
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

        let gardener = support::TreeGardener{ node: tree.root().into_raw() };
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

        let gardener = support::TreeGardener{ node: tree.root().into_raw() };
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

        let gardener = support::TreeGardener{ node: tree.root().into_raw() };
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

        let gardener = support::TreeGardener{ node: tree.root().into_raw() };
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

        let gardener = support::TreeGardener{ node: tree.root().into_raw().first_child().unwrap() };
        
        let lhs = gardener.pick_token(11.into());
        let rhs = gardener.pick_token(12.into());

        let anscestor = gardener.common_anscestor(lhs, rhs, syntax_kind::SEMI);
        assert_eq!(Some(syntax_kind::selcollist), anscestor.map(|x| engine.parsing_rules.from_kind_id(x.kind())));
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

        let rebuilded_source = rebuild_source(tree.root().token_at_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_byte_offset: 10,
            old_byte_len: 0,
            new_byte_len: 3,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_single_statement_with_inserting.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_offset(0));
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

        let rebuilded_source = rebuild_source(tree.root().token_at_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_byte_offset: 10,
            old_byte_len: 3,
            new_byte_len: 0,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_single_statement_with_deleting.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_offset(0));
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

        let rebuilded_source = rebuild_source(tree.root().token_at_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_byte_offset: 7,
            old_byte_len: 14,
            new_byte_len: 14,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_single_with_cross_over_2_statements.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_offset(0));
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

        let rebuilded_source = rebuild_source(tree.root().token_at_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_byte_offset: 13,
            old_byte_len: 0,
            new_byte_len: 24,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_append_statment.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_offset(0));
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

        let rebuilded_source = rebuild_source(tree.root().token_at_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_byte_offset: 0,
            old_byte_len: 0,
            new_byte_len: 24,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_prepend_statment.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_offset(0));
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

        let rebuilded_source = rebuild_source(tree.root().token_at_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_byte_offset: 31,
            old_byte_len: 2,
            new_byte_len: 26,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_with_maltibyte_char.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_offset(0));
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

        let rebuilded_source = rebuild_source(tree.root().token_at_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_byte_offset: 0,
            old_byte_len: 23,
            new_byte_len: 0,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_remove_all.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_offset(0));
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

        let rebuilded_source = rebuild_source(tree.root().token_at_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_byte_offset: 0,
            old_byte_len: 0,
            new_byte_len: 26,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_insert_from_empty.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_offset(0));
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

        let rebuilded_source = rebuild_source(tree.root().token_at_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_byte_offset: 9,
            old_byte_len: 2,
            new_byte_len: 17,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };

        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;
        let new_tree = tree.apply_batches(batches);
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_split_statement_on_inserting_semicolon.json"))?;

        let rebuilded_source = rebuild_source(new_tree.root().token_at_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }
    // FIXME: fn test_parse_concat_statement_on_removing_semicolon() // SELECT 1; SELECT 2; -> SELECT 1 SELECT 2;
    // FIXME: fn test_parse_brolken_keyword() // SELECT -> ELECT
    // FIXME: fn test_parse_keyword_only_with_semicolon() // SELECT -> SELECT;
}