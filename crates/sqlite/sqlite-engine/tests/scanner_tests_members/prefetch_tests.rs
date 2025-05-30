use std::collections::VecDeque;

use engine_core::scanner_engine::ScanEvent;
use scanner_core::{LookaheadIterator, Scanner, Token};
use sqlite_engine::syntax_kind;



#[test]
fn test_prefetch_all() -> Result<(), anyhow::Error> {
    let source = "SELECT 20 42 FROM foo x;SELECT 'xyz'; ";
    let engine = sqlite_engine::create()?;
    let mut scanner = Scanner::create(source, 0, engine.scanning_rules)?;

    scanner.shift();
    scanner.shift();
    
    let terminate_symbol = engine.parsing_rules.full_emit_config().to_symbol;
    let lookaheads = scanner.prefetch(terminate_symbol);

    let expected_lookaheads = VecDeque::from([
        Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 10, len: 2, value: Some("42".into()) },
            trailing_trivia: Some(vec![
                ScanEvent{ kind: syntax_kind::SPACE, offset: 12, len: 1, value: Some(" ".into()) }
            ])
        },
        Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::FROM, offset: 13, len: 4, value: Some("FROM".into()) },
            trailing_trivia: Some(vec![
                ScanEvent{ kind: syntax_kind::SPACE, offset: 17, len: 1, value: Some(" ".into()) }
            ])
        },
        Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::ID, offset: 18, len: 3, value: Some("foo".into()) },
            trailing_trivia: Some(vec![
                ScanEvent{ kind: syntax_kind::SPACE, offset: 21, len: 1, value: Some(" ".into()) }
            ])
        },
        Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::ID, offset: 22, len: 1, value: Some("x".into()) },
            trailing_trivia: None
        },
        Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::SEMI, offset: 23, len: 1, value: Some(";".into()) },
            trailing_trivia: None
        },
        Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::SELECT, offset: 24, len: 6, value: Some("SELECT".into()) },
            trailing_trivia: Some(vec![
                ScanEvent{ kind: syntax_kind::SPACE, offset: 30, len: 1, value: Some(" ".into()) }
            ])
        },
        Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::STRING, offset: 31, len: 5, value: Some("'xyz'".into()) },
            trailing_trivia: None
        },
        Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::SEMI, offset: 36, len: 1, value: Some(";".into()) },
            trailing_trivia: Some(vec![
                ScanEvent{ kind: syntax_kind::SPACE, offset: 37, len: 1, value: Some(" ".into()) }
            ])
        },
        Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::EOF, offset: 38, len: 0, value: None },
            trailing_trivia: None
        },
    ]);
    let expect_iter = LookaheadIterator::new(&expected_lookaheads, expected_lookaheads.len());
    
    assert_eq!(expect_iter.clone().count(), lookaheads.clone().count());
    assert_eq!(expect_iter, lookaheads);
    assert_eq!(expected_lookaheads.get(0), scanner.lookahead());
    Ok(())
}

#[test]
fn test_prefetch_statement() -> Result<(), anyhow::Error> {
    let source = "SELECT 20 42 FROM foo x;SELECT 'xyz'; ";
    let engine = sqlite_engine::create()?;
    let mut scanner = Scanner::create(source, 0, engine.scanning_rules)?;

    scanner.shift();
    scanner.shift();

    let terminate_symbol = engine.parsing_rules.statement_emit_config().to_symbol;
    let lookaheads = scanner.prefetch(terminate_symbol);

    let expected_lookaheads = VecDeque::from([
        Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 10, len: 2, value: Some("42".into()) },
            trailing_trivia: Some(vec![
                ScanEvent{ kind: syntax_kind::SPACE, offset: 12, len: 1, value: Some(" ".into()) }
            ])
        },
        Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::FROM, offset: 13, len: 4, value: Some("FROM".into()) },
            trailing_trivia: Some(vec![
                ScanEvent{ kind: syntax_kind::SPACE, offset: 17, len: 1, value: Some(" ".into()) }
            ])
        },
        Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::ID, offset: 18, len: 3, value: Some("foo".into()) },
            trailing_trivia: Some(vec![
                ScanEvent{ kind: syntax_kind::SPACE, offset: 21, len: 1, value: Some(" ".into()) }
            ])
        },
        Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::ID, offset: 22, len: 1, value: Some("x".into()) },
            trailing_trivia: None
        },
        Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::SEMI, offset: 23, len: 1, value: Some(";".into()) },
            trailing_trivia: None
        },
    ]);
    let expect_iter = LookaheadIterator::new(&expected_lookaheads, expected_lookaheads.len());
    
    assert_eq!(expect_iter.clone().count(), lookaheads.clone().count());
    assert_eq!(expect_iter, lookaheads);
    assert_eq!(expected_lookaheads.get(0), scanner.lookahead());
    Ok(())
}

#[test]
fn test_prefetch_statement_twice() -> Result<(), anyhow::Error> {
    let source = "SELECT 20 42 FROM foo x;SELECT 'xyz'; ";
    let engine = sqlite_engine::create()?;
    let mut scanner = Scanner::create(source, 0, engine.scanning_rules)?;

    scanner.shift();
    scanner.shift();

    let terminate_symbol = engine.parsing_rules.statement_emit_config().to_symbol;
    scanner.prefetch(terminate_symbol);
    let lookaheads = scanner.prefetch(terminate_symbol);

    let expected_lookaheads = VecDeque::from([
        Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 10, len: 2, value: Some("42".into()) },
            trailing_trivia: Some(vec![
                ScanEvent{ kind: syntax_kind::SPACE, offset: 12, len: 1, value: Some(" ".into()) }
            ])
        },
        Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::FROM, offset: 13, len: 4, value: Some("FROM".into()) },
            trailing_trivia: Some(vec![
                ScanEvent{ kind: syntax_kind::SPACE, offset: 17, len: 1, value: Some(" ".into()) }
            ])
        },
        Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::ID, offset: 18, len: 3, value: Some("foo".into()) },
            trailing_trivia: Some(vec![
                ScanEvent{ kind: syntax_kind::SPACE, offset: 21, len: 1, value: Some(" ".into()) }
            ])
        },
        Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::ID, offset: 22, len: 1, value: Some("x".into()) },
            trailing_trivia: None
        },
        Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::SEMI, offset: 23, len: 1, value: Some(";".into()) },
            trailing_trivia: None
        },
    ]);
    let expect_iter = LookaheadIterator::new(&expected_lookaheads, expected_lookaheads.len());
    
    assert_eq!(expect_iter.clone().count(), lookaheads.clone().count());
    assert_eq!(expect_iter, lookaheads);
    assert_eq!(expected_lookaheads.get(0), scanner.lookahead());
    Ok(())
}