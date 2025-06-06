use std::collections::VecDeque;

use engine_core::scanner_engine::ScanEvent;
use scanner_core::{iter::LookaheadIterator, Scanner, Token};
use sqlite_engine::syntax_kind;


mod prefetch_token_tests {
    use scanner_core::ScannerAccess;

    use super::*;

    #[test]
    fn test_prefetch_all() -> Result<(), anyhow::Error> {
        let source = "SELECT 20 42 FROM foo x;SELECT 'xyz'; ";
        let engine = sqlite_engine::create()?;
        let mut scanner = Scanner::create(source, 0, engine.scanning_rules)?;

        scanner.shift();
        scanner.shift();
        
        let terminate_symbol = engine.parsing_rules.full_emit_config().to_symbol;
        let lookaheads = scanner.prefetch_iter(terminate_symbol);

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
        let expect_iter = LookaheadIterator::new(&expected_lookaheads, 0, expected_lookaheads.len());
        
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
        let lookaheads = scanner.prefetch_iter(terminate_symbol);

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
        let expect_iter = LookaheadIterator::new(&expected_lookaheads, 0, expected_lookaheads.len());
        
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
        scanner.prefetch_iter(terminate_symbol);
        let lookaheads = scanner.prefetch_iter(terminate_symbol);

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
        let expect_iter = LookaheadIterator::new(&expected_lookaheads, 0, expected_lookaheads.len());
        
        assert_eq!(expect_iter.clone().count(), lookaheads.clone().count());
        assert_eq!(expect_iter, lookaheads);
        assert_eq!(expected_lookaheads.get(0), scanner.lookahead());
        Ok(())
    }
}

mod pregetch_stmt_tests {
    use scanner_core::ScannerAccess;

    use super::*;

    #[test]
    fn test_prefetch_overall_statement() -> Result<(), anyhow::Error> {
        let source = "SELECT * FROM foo;";
        let engine = sqlite_engine::create()?;
        let scanner = Scanner::create(source, 0, engine.scanning_rules)?;
        let stmt_scanners = scanner.statement_scanners(syntax_kind::SEMI).collect::<Vec<_>>();
        assert_eq!(2, stmt_scanners.len());

        'prefetch: {
            let mut view = stmt_scanners[0].as_view(0..30);
            let iter = view.prefetch_iter(syntax_kind::SEMI);
            assert_eq!(5, iter.len());
            assert_eq!(5, iter.count());
            break 'prefetch;
        }
        'prefetch: {
            // EOF only
            let mut view = stmt_scanners[1].as_view(0..30);
            let iter = view.prefetch_iter(syntax_kind::SEMI);
            assert_eq!(1, iter.len());
            assert_eq!(1, iter.count());
            break 'prefetch;
        }
        Ok(())
    }

    #[test]
    fn test_prefetch_inside_statement() -> Result<(), anyhow::Error> {
        let source = "SELECT * FROM foo;";
        let engine = sqlite_engine::create()?;
        let scanner = Scanner::create(source, 0, engine.scanning_rules)?;
        let stmt_scanners = scanner.statement_scanners(syntax_kind::SEMI).collect::<Vec<_>>();
        assert_eq!(2, stmt_scanners.len());

        'prefetch: {
            let mut view = stmt_scanners[0].as_view(7..10);
            let iter = view.prefetch_iter(syntax_kind::SEMI);
            assert_eq!(2, iter.len());

            let lookaheads = iter.collect::<Vec<_>>();
            assert_eq!(2, lookaheads.len());
            assert_eq!(syntax_kind::STAR, lookaheads[0].main.kind);
            assert_eq!(syntax_kind::FROM, lookaheads[1].main.kind);
            break 'prefetch;
        }
        'prefetch: {
            // EOF only
            let mut view = stmt_scanners[1].as_view(7..10);
            let iter = view.prefetch_iter(syntax_kind::SEMI);
            assert_eq!(0, iter.len());
            assert_eq!(0, iter.count());
            break 'prefetch;
        }

        Ok(())
    }

    #[test]
    fn test_prefetch_cross_over_2_statements() -> Result<(), anyhow::Error> {
        let source = "SELECT * FROM foo;SELECT 42;";
        let engine = sqlite_engine::create()?;
        let scanner = Scanner::create(source, 0, engine.scanning_rules)?;
        let stmt_scanners = scanner.statement_scanners(syntax_kind::SEMI).collect::<Vec<_>>();
        assert_eq!(3, stmt_scanners.len());

        'prefetch: {
            let mut view = stmt_scanners[0].as_view(10..26);
            let iter = view.prefetch_iter(syntax_kind::SEMI);
            assert_eq!(3, iter.len());

            let lookaheads = iter.collect::<Vec<_>>();
            assert_eq!(3, lookaheads.len());
            assert_eq!(syntax_kind::FROM, lookaheads[0].main.kind);
            assert_eq!(syntax_kind::ID, lookaheads[1].main.kind);
            assert_eq!(syntax_kind::SEMI, lookaheads[2].main.kind);
            break 'prefetch;
        }
        'prefetch: {
            let mut view = stmt_scanners[1].as_view(10..26);
            let iter = view.prefetch_iter(syntax_kind::SEMI);
            assert_eq!(2, iter.len());

            let lookaheads = iter.collect::<Vec<_>>();
            assert_eq!(2, lookaheads.len());
            assert_eq!(syntax_kind::SELECT, lookaheads[0].main.kind);
            assert_eq!(syntax_kind::INTEGER, lookaheads[1].main.kind);
            break 'prefetch;
        }
        'prefetch: {
            // EOF only
            let mut view = stmt_scanners[2].as_view(10..26);
            let iter = view.prefetch_iter(syntax_kind::SEMI);
            assert_eq!(0, iter.len());
            assert_eq!(0, iter.count());
            break 'prefetch;
        }

        Ok(())
    }

    #[test]
    fn test_prefetch_cross_over_3_statements() -> Result<(), anyhow::Error> {
        let source = "SELECT * FROM foo;SELECT 42;  SELECT 'foo' FROM x;";
        let engine = sqlite_engine::create()?;
        let scanner = Scanner::create(source, 0, engine.scanning_rules)?;
        let stmt_scanners = scanner.statement_scanners(syntax_kind::SEMI).collect::<Vec<_>>();
        assert_eq!(4, stmt_scanners.len());

        'prefetch: {
            let mut view = stmt_scanners[0].as_view(17..33);
            let iter = view.prefetch_iter(syntax_kind::SEMI);
            assert_eq!(1, iter.len());

            let lookaheads = iter.collect::<Vec<_>>();
            assert_eq!(1, lookaheads.len());
            assert_eq!(syntax_kind::SEMI, lookaheads[0].main.kind);
            break 'prefetch;
        }
        'prefetch: {
            let mut view = stmt_scanners[1].as_view(17..33);
            let iter = view.prefetch_iter(syntax_kind::SEMI);
            assert_eq!(3, iter.len());

            let lookaheads = iter.collect::<Vec<_>>();
            assert_eq!(3, lookaheads.len());
            assert_eq!(syntax_kind::SELECT, lookaheads[0].main.kind);
            assert_eq!(syntax_kind::INTEGER, lookaheads[1].main.kind);
            assert_eq!(syntax_kind::SEMI, lookaheads[2].main.kind);
            break 'prefetch;
        }
        'prefetch: {
            let mut view = stmt_scanners[2].as_view(17..33);
            let iter = view.prefetch_iter(syntax_kind::SEMI);
            assert_eq!(1, iter.len());

            let lookaheads = iter.collect::<Vec<_>>();
            assert_eq!(1, lookaheads.len());
            assert_eq!(syntax_kind::SELECT, lookaheads[0].main.kind);
            break 'prefetch;
        }

        Ok(())
    }
}