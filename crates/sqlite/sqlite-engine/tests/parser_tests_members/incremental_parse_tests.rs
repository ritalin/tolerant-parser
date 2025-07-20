use tolerant_parser_sdk::core::parser_core::{incremental::EditScope, Parser};
use sqlite_engine::syntax_kind;
use tolerant_parser_sdk::core::engine_core::scanner_engine::CaseSensitivity;
use tolerant_parser_sdk::core::parser_core::{ParserConfig, ParseMode, RecoveryPenalty};

mod incremental_support_tests {
    use tolerant_parser_sdk::core::parser_core::incremental::support;

    use super::*;

    #[test]
    fn text_prev_token() -> Result<(), anyhow::Error> {
        let source = "SELECT 101 AS x FROM foo u;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let root_node = tree.root();
        let gardener = support::TreeGardener::new(&root_node);
        let token = gardener.pick_token(11);
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
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let root_node = tree.root();
        let gardener = support::TreeGardener::new(&root_node);
        let token = gardener.pick_token(10);
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
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let root_node = tree.root();
        let gardener = support::TreeGardener::new(&root_node);
        let token = gardener.pick_token(26);
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
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let root_node = tree.root();
        let gardener = support::TreeGardener::new(&root_node);
        let token = gardener.pick_token(11);
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
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let root_node = tree.root();
        let gardener = support::TreeGardener::new(&root_node);
        let token = gardener.pick_token(11);
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
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let root_node = tree.root();
        let gardener = support::TreeGardener::new(&root_node);
        let token = gardener.pick_token(26);
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
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let stmt_node = tree.root().nth_child(0).and_then(|el| el.to_node()).unwrap();
        let gardener = support::TreeGardener::new(&stmt_node);
        
        let lhs = gardener.pick_token(11);
        let rhs = gardener.pick_token(12);

        let anscestor = gardener.common_anscestor(lhs, rhs, syntax_kind::SEMI);
        assert_eq!(Some(syntax_kind::selcollist), anscestor.map(|x| engine.parsing_rules.from_kind_id(x.node.kind())));
        Ok(())
    }
}

mod parser_tests {
    use tolerant_parser_sdk::core::parser_core::{ParseMode, ParserConfig, RecoveryPenalty};
    use crate::test_support::{self, ExpectNode, };
    use super::*;

    #[test]
    fn test_parse_single_statement_with_inserting() -> Result<(), anyhow::Error> {
        let source = "SELECT 42 x FROM foo u;";
        let new_source = "SELECT 42 AS x FROM foo u;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let rebuilded_source = test_support::rebuild_source(tree.root().token_at_utf16_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_char_offset: 10,
            old_char_len: 0,
            new_char_len: 3,
            text: "AS ".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_single_statement_with_inserting.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_single_statement_with_deleting() -> Result<(), anyhow::Error> {
        let source =     "SELECT 42 AS x FROM foo u;";
        let new_source = "SELECT 42 x FROM foo u;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let rebuilded_source = test_support::rebuild_source(tree.root().token_at_utf16_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_char_offset: 10,
            old_char_len: 3,
            new_char_len: 0,
            text: "".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_single_statement_with_deleting.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_single_with_cross_over_2_statements() -> Result<(), anyhow::Error> {
        let source = "SELECT '101'; SELECT 42 x FROM foo u;";
        let new_source = "SELECT 42; SELECT p, 42 x FROM foo u;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let rebuilded_source = test_support::rebuild_source(tree.root().token_at_utf16_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_char_offset: 7,
            old_char_len: 14,
            new_char_len: 14,
            text: "42; SELECT p, ".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_single_with_cross_over_2_statements.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_append_statment() -> Result<(), anyhow::Error> {
        let source = "SELECT '101';";
        let new_source = "SELECT '101'; SELECT 42 x FROM foo u;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let rebuilded_source = test_support::rebuild_source(tree.root().token_at_utf16_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_char_offset: 13,
            old_char_len: 0,
            new_char_len: 24,
            text: " SELECT 42 x FROM foo u;".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_append_statment.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    // #[test]
    // fn test_parse_prepend_statment() -> Result<(), anyhow::Error> {
    //     let source = "SELECT '101';";
    //     let new_source = " SELECT 42 x FROM foo u;SELECT '101';";

    //     let engine = sqlite_engine::create()?;
    //     let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
    //     let parser = Parser::new(engine.clone(), config.clone());
    //     let tree = parser.parse(source)?;

    //     let rebuilded_source = test_support::rebuild_source(tree.root().token_at_utf16_offset(0));
    //     assert_eq!(source, rebuilded_source);

    //     let scope = EditScope{
    //         start_char_offset: 0,
    //         old_char_len: 0,
    //         new_char_len: 24,
    //     };

    //     let new_tree = parser.parse_incremental(&tree, &[scope])?;
    //     let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_prepend_statment.json"))?;

    //     let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
    //     assert_eq!(new_source, rebuilded_source);

    //     test_support::verify(new_tree.root(), &expect_node);

    //     Ok(())
    // }

    #[test]
    fn test_parse_with_maltibyte_char() -> Result<(), anyhow::Error> {
        let source = "/* 日本語コメント */SELECT 42 AS a;";
        let new_source = "/* 日本語コメント */SELECT 42 /* ASを取り除いた */ a;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let rebuilded_source = test_support::rebuild_source(tree.root().token_at_utf16_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_char_offset: 23,
            old_char_len: 2,
            new_char_len: 14,
            text: "/* ASを取り除いた */".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_with_maltibyte_char.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_remove_all() -> Result<(), anyhow::Error> {
        let source = "SELECT 42 x FROM foo u;";
        let new_source = "";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let rebuilded_source = test_support::rebuild_source(tree.root().token_at_utf16_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 23,
            new_char_len: 0,
            text: "".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_remove_all.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_insert_from_empty() -> Result<(), anyhow::Error> {
        let source = "";
        let new_source = "SELECT 42 AS x FROM foo u;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let rebuilded_source = test_support::rebuild_source(tree.root().token_at_utf16_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 26,
            text: "SELECT 42 AS x FROM foo u;".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_insert_from_empty.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_insert_changining_full() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 2;";
        let new_source = "SELECT 11;SELECT 22;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let rebuilded_source = test_support::rebuild_source(tree.root().token_at_utf16_offset(0));
        assert_eq!(source, rebuilded_source);

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 18,
            new_char_len: 20,
            text: "SELECT 11;SELECT 22;".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_insert_changining_full.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    // #[test]
    // fn test_parse_split_statement_on_inserting_semicolon() -> Result<(), anyhow::Error> {
    //     let source = "SELECT 1 AS x;";
    //     let new_source = "SELECT 1 AS y; SELECT 2 AS x;";

    //     let engine = sqlite_engine::create()?;
    //     let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
    //     let parser = Parser::new(engine.clone(), config.clone());
    //     let tree = parser.parse(source)?;

    //     let rebuilded_source = test_support::rebuild_source(tree.root().token_at_utf16_offset(0));
    //     assert_eq!(source, rebuilded_source);

    //     let scope = EditScope{
    //         start_char_offset: 9,
    //         old_char_len: 2,
    //         new_char_len: 17,
    //     };

    //     let new_tree = parser.parse_incremental(&tree, &[scope])?;
    //     let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_split_statement_on_inserting_semicolon.json"))?;

    //     let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
    //     assert_eq!(new_source, rebuilded_source);

    //     test_support::verify(new_tree.root(), &expect_node);

    //     Ok(())
    // }

    #[test]
    fn test_parse_broken_keyword_by_inserting() -> Result<(), anyhow::Error> {
        let source = "S";
        let new_source = "SE";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 1,
            old_char_len: 0,
            new_char_len: 1,
            text: "E".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_broken_keyword_by_inserting.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_broken_keyword_by_removing_first_char() -> Result<(), anyhow::Error> {
        let source = "SELECT";
        let new_source = "ELECT";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 1,
            new_char_len: 0,
            text: "".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_broken_keyword_by_removing_first_char.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_broken_keyword_by_inserting_first() -> Result<(), anyhow::Error> {
        let source = "ELECT";
        let new_source = "SELECT";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 0,
            new_char_len: 1,
            text: "S".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_broken_keyword_by_inserting_first.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_statement_by_appending_semicolon() -> Result<(), anyhow::Error> {
        let source = "SELECT 42";
        let new_source = "SELECT 42;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 9,
            old_char_len: 0,
            new_char_len: 1,
            text: ";".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_statement_by_appending_semicolon.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_statement_by_appending_leading_trivia() -> Result<(), anyhow::Error> {
        let source = "SELECT 42";
        let new_source = "SELECT 42/* Answer to the Ultimate Question of Life, the Universe, and Everything */";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 9,
            old_char_len: 0,
            new_char_len: 75,
            text: "/* Answer to the Ultimate Question of Life, the Universe, and Everything */".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_statement_by_appending_leading_trivia.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_statement_by_dropping_semicolon() -> Result<(), anyhow::Error> {
        let source = "SELECT 42;";
        let new_source = "SELECT 42";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 9,
            old_char_len: 1,
            new_char_len: 0,
            text: "".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_statement_by_dropping_semicolon.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_statement_by_dropping_first_statement() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 42;";
        let new_source = "SELECT 42;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 9,
            new_char_len: 0,
            text: "".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_statement_by_dropping_first_statement.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_statement_by_dropping_first_statement_without_trailing_trivia() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;\nSELECT 42;";
        let new_source = "\nSELECT 42;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 9,
            new_char_len: 0,
            text: "".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_statement_by_dropping_first_statement_without_trailing_trivia.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_statement_by_dropping_first_statement_without_leading_trivia() -> Result<(), anyhow::Error> {
        let source = "/* comment */SELECT 1;\nSELECT 2;\n";
        let new_source = "/* comment */SELECT 2;\n";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive:CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 13,
            old_char_len: 10,
            new_char_len: 0,
            text: "".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_statement_by_dropping_first_statement_without_leading_trivia.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_statement_by_dropping_middle_statement() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;\nSELECT 2;\nSELECT 3;\nSELECT 4;\n";
        let new_source = "SELECT 1;\nSELECT 4;\n";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 10,
            old_char_len: 20,
            new_char_len: 0,
            text: "".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_statement_by_dropping_middle_statement.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_statement_by_dropping_middle_statement_without_trailing_trivia() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;\nSELECT 2;\nSELECT 3;\nSELECT 4;\n";
        let new_source = "SELECT 1;\n\nSELECT 4;\n";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 10,
            old_char_len: 19,
            new_char_len: 0,
            text: "".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_statement_by_dropping_middle_statement_without_trailing_trivia.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_statement_by_dropping_middle_statement_without_leading_trivia() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;\n/* comment */SELECT 2;\nSELECT 3;\nSELECT 4;\n";
        let new_source = "SELECT 1;\n/* comment */\nSELECT 4;\n";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 23,
            old_char_len: 19,
            new_char_len: 0,
            text: "".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_statement_by_dropping_middle_statement_without_leading_trivia.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_statement_by_dropping_last_statement() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;\nSELECT 2;\nSELECT 3;\nSELECT 4;\n";
        let new_source = "SELECT 1;\nSELECT 2;\n";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 20,
            old_char_len: 20,
            new_char_len: 0,
            text: "".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_statement_by_dropping_last_statement.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_statement_by_dropping_last_statement_without_trailing_trivia() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;\nSELECT 2;\nSELECT 3;\nSELECT 4;\n";
        let new_source = "SELECT 1;\nSELECT 2;\n\n";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 20,
            old_char_len: 19,
            new_char_len: 0,
            text: "".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_statement_by_dropping_last_statement_without_trailing_trivia.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_statement_by_dropping_last_statement_without_leading_trivia() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;\n/* comment */SELECT 2;\nSELECT 3;\nSELECT 4;\n";
        let new_source = "SELECT 1;\n/* comment */";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 23,
            old_char_len: 30,
            new_char_len: 0,
            text: "".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_statement_by_dropping_last_statement_without_leading_trivia.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    // #[test]
    // fn test_parse_statement_by_filling_value() -> Result<(), anyhow::Error> {
    //     let source = "SELECT ";
    //     let new_source = "SELECT 42;";

    //     let engine = sqlite_engine::create()?;
    //     let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
    //     let parser = Parser::new(engine.clone(), config.clone());
    //     let tree = parser.parse(source)?;

    //     let scope = EditScope{
    //         start_char_offset: 7,
    //         old_char_len: 0,
    //         new_char_len: 2,
    //     };

    //     let new_tree = parser.parse_incremental(&tree, &[scope])?;
    //     let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_statement_by_filling_value.json"))?;

    //     let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
    //     assert_eq!(new_source, rebuilded_source);

    //     test_support::verify(new_tree.root(), &expect_node);

    //     Ok(())
    // }

    #[test]
    fn test_parse_statement_by_replacing_all() -> Result<(), anyhow::Error> {
        let source = "SELECT 42;";
        let new_source = "SELECT 42 AS  FRO;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 0,
            old_char_len: 10,
            new_char_len: 18,
            text: "SELECT 42 AS  FRO;".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_statement_by_replacing_all.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_statement_by_editing_latter() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 2;";
        let new_source = "SELECT 1;SELECT 23;";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 17,
            old_char_len: 0,
            new_char_len: 1,
            text: "3".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_statement_by_editing_latter.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_statement_with_semicolon_after_newline() -> Result<(), anyhow::Error> {
        let source = "SELECT 42;";
        let new_source = "SELECT 42;\n";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 10,
            old_char_len: 0,
            new_char_len: 1,
            text: "\n".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_statement_with_semicolon_after_newline.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_statement_following_incorrent_statement() -> Result<(), anyhow::Error> {
        let source = ";";
        let new_source = ";S";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 1,
            old_char_len: 0,
            new_char_len: 1,
            text: "S".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_statement_following_incorrent_statement.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }

    #[test]
    fn test_parse_incorrect_identifier_removing_word() -> Result<(), anyhow::Error> {
        let source = "/* こめ";
        let new_source = "/* こ";

        let engine = sqlite_engine::create()?;
        let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
        let parser = Parser::new(engine.clone(), config.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_char_offset: 4,
            old_char_len: 1,
            new_char_len: 0,
            text: "".into(),
        };

        let new_tree = parser.parse_incremental(&tree, &[scope])?;
        let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/parser_tests_members/test_parse_incorrect_identifier_removing_word.json"))?;

        let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
        assert_eq!(new_source, rebuilded_source);

        test_support::verify(new_tree.root(), &expect_node);

        Ok(())
    }
}