#![cfg(not(engine_ungenerated))]

use parser_core::Parser;

#[path = "supports/test_support.rs"]
mod test_support;
use test_support::*;

mod parser_tests_members {
    mod parse_dispatcher_tests;
    mod node_handler_tests;
    mod parse_recovery_tests;
    mod incrementa_parse_tests;
    mod syntax_tree_tests;
}

#[test]
fn test_const_select() -> Result<(), anyhow::Error> {
    let source = "SELECT 42;";
    
    let engine = sqlite_engine::create()?;
    let parsing_rules = engine.parsing_rules;
    let parser = Parser::new(engine);

    let tree = parser.parse(source)?;
    let expect_node = serde_json::from_str::<ExpectNode>(include_str!("fixtures/parse_tests/test_const_select.json"))?;

    test_support::verify(&ActualNode::Node(tree.root()), &expect_node, parsing_rules, 0);
    Ok(())
}

#[test]
fn test_star_select() -> Result<(), anyhow::Error> {
    let source = "SELECT * FROM foo;";

    let engine = sqlite_engine::create()?;
    let parsing_rules = engine.parsing_rules;
    let parser = Parser::new(engine);

    let tree = parser.parse(source)?;
    let expect_node = serde_json::from_str::<ExpectNode>(include_str!("fixtures/parse_tests/test_star_select.json"))?;

    test_support::verify(&ActualNode::Node(tree.root()), &expect_node, parsing_rules, 0);
    Ok(())
}

#[test]
fn test_meny_select_statements() -> Result<(), anyhow::Error> {
    let source = r#"
    SELECT * FROM foo;
    SELECT 101;
    "#;

    let engine = sqlite_engine::create()?;
    let parsing_rules = engine.parsing_rules;
    let parser = Parser::new(engine);

    let tree = parser.parse(source)?;
    let expect_node = serde_json::from_str::<ExpectNode>(include_str!("fixtures/parse_tests/test_meny_select_statements.json"))?;

    test_support::verify(&ActualNode::Node(tree.root()), &expect_node, parsing_rules, 0);
    Ok(())
}