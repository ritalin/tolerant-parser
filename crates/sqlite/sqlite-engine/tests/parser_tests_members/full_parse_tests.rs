use parser_core::{Parser, support::test_support};

#[test]
fn test_const_select() -> Result<(), anyhow::Error> {
    let source = "SELECT 42;";
    
    let engine = sqlite_engine::create()?;
    let parser = Parser::new(engine);

    let tree = parser.parse(source)?;
    let expect_node = serde_json::from_str::<Vec<test_support::ExpectNode>>(include_str!("../fixtures/parse_tests/full_parse_tests/test_const_select.json"))?;

    test_support::verify(tree.root(), &expect_node);
    Ok(())
}

#[test]
fn test_star_select() -> Result<(), anyhow::Error> {
    let source = "SELECT * FROM foo;";

    let engine = sqlite_engine::create()?;
    let parser = Parser::new(engine);

    let tree = parser.parse(source)?;
    let expect_node = serde_json::from_str::<Vec<test_support::ExpectNode>>(include_str!("../fixtures/parse_tests/full_parse_tests/test_star_select.json"))?;

    test_support::verify(tree.root(), &expect_node);
    Ok(())
}

#[test]
fn test_meny_select_statements() -> Result<(), anyhow::Error> {
    let source = r#"
    SELECT * FROM foo;
    SELECT 101;
    "#;

    let engine = sqlite_engine::create()?;
    let parser = Parser::new(engine);

    let tree = parser.parse(source)?;
    let expect_node = serde_json::from_str::<Vec<test_support::ExpectNode>>(include_str!("../fixtures/parse_tests/full_parse_tests/test_meny_select_statements.json"))?;

    test_support::verify(tree.root(), &expect_node);
    Ok(())
}

#[test]
fn test_incomplete_statement() -> Result<(), anyhow::Error> {
    let source = "SELECT;";

    let engine = sqlite_engine::create()?;
    let parser = Parser::new(engine);

    let tree = parser.parse(source)?;
    let expect_node = serde_json::from_str::<Vec<test_support::ExpectNode>>(include_str!("../fixtures/parse_tests/full_parse_tests/test_incomplete_statement.json"))?;

    test_support::verify(tree.root(), &expect_node);
    Ok(())
}

#[test]
fn test_parse_with_surrogate_pair_literal() -> Result<(), anyhow::Error> {
    let source = "SELECT '𩸽' as s;";

    let engine = sqlite_engine::create()?;
    let parser = Parser::new(engine);

    let tree = parser.parse(source)?;
    let expect_node = serde_json::from_str::<Vec<test_support::ExpectNode>>(include_str!("../fixtures/parse_tests/full_parse_tests/test_parse_with_surrogate_pair_literal.json"))?;

    test_support::verify(tree.root(), &expect_node);
    Ok(())
}

#[test]
fn test_parse_with_incorrect_literal() -> Result<(), anyhow::Error> {
    let source = "SELECT '";

    let engine = sqlite_engine::create()?;
    let parser = Parser::new(engine);

    let tree = parser.parse(source)?;
    let expect_node = serde_json::from_str::<Vec<test_support::ExpectNode>>(include_str!("../fixtures/parse_tests/full_parse_tests/test_parse_with_incorrect_literal.json"))?;

    test_support::verify(tree.root(), &expect_node);
    Ok(())
}

#[test]
fn test_parse_with_incorrect_literal_2() -> Result<(), anyhow::Error> {
    let source = "SELECT 'a";

    let engine = sqlite_engine::create()?;
    let parser = Parser::new(engine);

    let tree = parser.parse(source)?;
    let expect_node = serde_json::from_str::<Vec<test_support::ExpectNode>>(include_str!("../fixtures/parse_tests/full_parse_tests/test_parse_with_incorrect_literal_2.json"))?;

    test_support::verify(tree.root(), &expect_node);
    Ok(())
}