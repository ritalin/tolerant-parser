use tolerant_parser_sdk::{core::{engine_core::scanner_engine::CaseSensitivity, parser_core::{incremental::EditScope, ParseMode, Parser, ParserConfig, RecoveryPenalty}}, support::test_support::{self, ExpectNode}};

#[test]
fn test_insert_word() -> Result<(), anyhow::Error> {
    let source = "SELECT '101'; SELECT x FROM foo u;";
    let new_source = "SELECT '101', 42; SELECT x, 42 FROM foo u;";

    let engine = sqlite_engine::create()?;
    let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
    let parser = Parser::new(engine.clone(), config.clone());
    let tree = parser.parse(source)?;

    let rebuilded_source = test_support::rebuild_source(tree.root().token_at_utf16_offset(0));
    assert_eq!(source, rebuilded_source);

    let scopes = vec![
        EditScope{
            start_char_offset: 12,
            old_char_len: 0,
            text: ", 42".into(),
        },
        EditScope{
            start_char_offset: 22,
            old_char_len: 0,
            text: ", 42".into(),
        },
    ];

    let new_tree = parser.parse_incremental(&tree, &scopes)?;

    let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
    assert_eq!(new_source, rebuilded_source);
    
    let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/multi_cursor_tests/test_insert_word.json"))?;
    test_support::verify(new_tree.root(), &expect_node);
    Ok(())
}

#[test]
fn test_drop_word() -> Result<(), anyhow::Error> {
    let source = "SELECT '101', 1; SELECT 2, x FROM foo u;";
    let new_source = "SELECT 1; SELECT 2;";

    let engine = sqlite_engine::create()?;
    let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
    let parser = Parser::new(engine.clone(), config.clone());
    let tree = parser.parse(source)?;

    let rebuilded_source = test_support::rebuild_source(tree.root().token_at_utf16_offset(0));
    assert_eq!(source, rebuilded_source);

    let scopes = vec![
        EditScope{
            start_char_offset: 7,
            old_char_len: 7,
            text: "".into(),
        },
        EditScope{
            start_char_offset: 25,
            old_char_len: 14,
            text: "".into(),
        },
    ];

    let new_tree = parser.parse_incremental(&tree, &scopes)?;

    let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
    assert_eq!(new_source, rebuilded_source);
    
    let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/multi_cursor_tests/test_drop_word.json"))?;
    test_support::verify(new_tree.root(), &expect_node);
    Ok(())
}

#[test]
fn test_replace_word() -> Result<(), anyhow::Error> {
    let source = "SELECT 1; SELECT 2; SELECT 3; SELECT 4;";
    let new_source = "SELECT 123; SELECT 101;";

    let engine = sqlite_engine::create()?;
    let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
    let parser = Parser::new(engine.clone(), config.clone());
    let tree = parser.parse(source)?;

    let rebuilded_source = test_support::rebuild_source(tree.root().token_at_utf16_offset(0));
    assert_eq!(source, rebuilded_source);

    let scopes = vec![
        EditScope{
            start_char_offset: 8,
            old_char_len: 19,
            text: "2".into(),
        },
        EditScope{
            start_char_offset: 37,
            old_char_len: 1,
            text: "101".into(),
        },
    ];

    let new_tree = parser.parse_incremental(&tree, &scopes)?;

    let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
    assert_eq!(new_source, rebuilded_source);

    let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/multi_cursor_tests/test_replace_word.json"))?;
    test_support::verify(new_tree.root(), &expect_node);
    Ok(())
}

#[test]
fn test_drap_and_move_word() -> Result<(), anyhow::Error> {
    let source = "SELECT 1, 42; SELECT 2;";
    let new_source = "SELECT 1; SELECT 2, 42;";

    let engine = sqlite_engine::create()?;
    let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
    let parser = Parser::new(engine.clone(), config.clone());
    let tree = parser.parse(source)?;

    let rebuilded_source = test_support::rebuild_source(tree.root().token_at_utf16_offset(0));
    assert_eq!(source, rebuilded_source);

    let scopes = vec![
        EditScope{
            start_char_offset: 8,
            old_char_len: 4,
            text: "".into(),
        },
        EditScope{
            start_char_offset: 22,
            old_char_len: 0,
            text: ", 42".into(),
        },
    ];

    let new_tree = parser.parse_incremental(&tree, &scopes)?;

    let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
    assert_eq!(new_source, rebuilded_source);

    let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/multi_cursor_tests/test_drap_and_move_word.json"))?;
    test_support::verify(new_tree.root(), &expect_node);
    Ok(())
}

#[test]
fn test_no_edit() -> Result<(), anyhow::Error> {
    let source = "SELECT 1; SELECT 2;";
    let new_source = "SELECT 1; SELECT 2;";

    let engine = sqlite_engine::create()?;
    let config = ParserConfig{ mode: ParseMode::ByStatement, penalty: RecoveryPenalty::default(), case_sensitive: CaseSensitivity::Insensitive };
    let parser = Parser::new(engine.clone(), config.clone());
    let tree = parser.parse(source)?;

    let rebuilded_source = test_support::rebuild_source(tree.root().token_at_utf16_offset(0));
    assert_eq!(source, rebuilded_source);

    let scopes = vec![
        EditScope{
            start_char_offset: 6,
            old_char_len: 0,
            text: "".into(),
        },
        EditScope{
            start_char_offset: 18,
            old_char_len: 0,
            text: "".into(),
        },
    ];

    let new_tree = parser.parse_incremental(&tree, &scopes)?;

    let rebuilded_source = test_support::rebuild_source(new_tree.root().token_at_utf16_offset(0));
    assert_eq!(new_source, rebuilded_source);
    
    let expect_node = serde_json::from_str::<Vec<ExpectNode>>(include_str!("../fixtures/parse_tests/multi_cursor_tests/test_no_edit.json"))?;
    test_support::verify(new_tree.root(), &expect_node);
    Ok(())
}