#![cfg(not(engine_ungenerated))]

use engine_core::scanner_engine::ScanEvent;
use scanner_core::{Scanner, Token};
use sqlite_engine::syntax_kind;

#[test]
fn test_peek_lookahead() -> Result<(), anyhow::Error> {
    let source = "INSERT OR REPLACE  INTO";
    let engine = sqlite_engine::create()?;
    let mut scanner = Scanner::create(source, 0, engine.scanning_rules)?;

    'peek: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::r#INSERT, offset: 0, len: 6, value: Some("INSERT".into()) },
            trailing_trivia: Some(vec![
                ScanEvent { kind: syntax_kind::r#SPACE, offset: 6, len: 1, value: Some(" ".into()) }
            ]),
        };
        assert_eq!(Some(&expect_token), scanner.lookahead());
        assert_eq!(Some(&expect_token), scanner.lookahead());
        assert_eq!(Some(&expect_token), scanner.lookahead());
        break 'peek;
    }
    'peek: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::r#OR, offset: 7, len: 2, value: Some("OR".into()) },
            trailing_trivia: Some(vec![
                ScanEvent { kind: syntax_kind::r#SPACE, offset: 9, len: 1, value: Some(" ".into()) }
            ]),
        };
        scanner.shift().as_ref();
        assert_eq!(Some(&expect_token), scanner.lookahead());
        assert_eq!(Some(&expect_token), scanner.lookahead());
        assert_eq!(Some(&expect_token), scanner.lookahead());
        break 'peek;
    }
    
    Ok(())
}

#[test]
fn test_shift_for_main_token_only() -> Result<(), anyhow::Error> {
    let source = "INSERT OR REPLACE  INTO";
    let engine = sqlite_engine::create()?;
    let mut scanner = Scanner::create(source, 0, engine.scanning_rules)?;

    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::r#INSERT, offset: 0, len: 6, value: Some("INSERT".into()) },
            trailing_trivia: Some(vec![
                ScanEvent { kind: syntax_kind::r#SPACE, offset: 6, len: 1, value: Some(" ".into()) }
            ]),
        };
        assert_eq!(Some(expect_token), scanner.shift());
        break 'scanning;
    }
    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::r#OR, offset: 7, len: 2, value: Some("OR".into()) },
            trailing_trivia: Some(vec![
                ScanEvent { kind: syntax_kind::r#SPACE, offset: 9, len: 1, value: Some(" ".into()) }
            ]),
        };
        assert_eq!(Some(expect_token), scanner.shift());
        break 'scanning;
    }
    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::r#REPLACE, offset: 10, len: 7, value: Some("REPLACE".into()) },
            trailing_trivia: Some(vec![
                ScanEvent { kind: syntax_kind::r#SPACE, offset: 17, len: 2, value: Some("  ".into()) }
            ]),
        };
        assert_eq!(Some(expect_token), scanner.shift());
        break 'scanning;
    }
    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::r#INTO, offset: 19, len: 4, value: Some("INTO".into()) },
            trailing_trivia: None,
        };
        assert_eq!(Some(expect_token), scanner.shift());
        break 'scanning;
    }
    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::r#EOF, offset: 23, len: 0, value: None },
            trailing_trivia: None,
        };
        assert_eq!(Some(expect_token), scanner.shift());
        break 'scanning;
    }
    'scanning: {
        assert_eq!(None, scanner.shift());
        break 'scanning;
    }
    Ok(())
}

#[test]
fn test_shift_with_leading_trivia() -> Result<(), anyhow::Error> {
    let source = "/* いろはにqwertyほへと */ INSERT OR --あいう";
    let engine = sqlite_engine::create()?;
    let mut scanner = Scanner::create(source, 0, engine.scanning_rules)?;

    'scanning: {
        let expect_token = Token {
            leading_trivia: Some(vec![
                ScanEvent { kind: syntax_kind::r#COMMENT, offset: 0, len: 33, value: Some("/* いろはにqwertyほへと */".into()) },
                ScanEvent { kind: syntax_kind::SPACE, offset: 33, len: 1, value: Some(" ".into()) }
            ]),
            main: ScanEvent { kind: syntax_kind::r#INSERT, offset: 34, len: 6, value: Some("INSERT".into()) },
            trailing_trivia: Some(vec![
                ScanEvent { kind: syntax_kind::r#SPACE, offset: 40, len: 1, value: Some(" ".into()) }
            ]),
        };
        assert_eq!(Some(expect_token), scanner.shift());
        break 'scanning;
    }
    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::r#OR, offset: 41, len: 2, value: Some("OR".into()) },
            trailing_trivia: Some(vec![
                ScanEvent { kind: syntax_kind::r#SPACE, offset: 43, len: 1, value: Some(" ".into()) }
            ]),
        };
        assert_eq!(Some(expect_token), scanner.shift());
        break 'scanning;
    }
    'scanning: {
        let expect_token = Token {
            leading_trivia: Some(vec![
                ScanEvent { kind: syntax_kind::r#COMMENT, offset: 44, len: 11, value: Some("--あいう".into()) },
            ]),
            main: ScanEvent { kind: syntax_kind::r#EOF, offset: 55, len: 0, value: None },
            trailing_trivia: None,
        };
        assert_eq!(Some(expect_token), scanner.shift());
        break 'scanning;
    }
    'scanning: {
        assert_eq!(None, scanner.shift());
        break 'scanning;
    }

    Ok(())
}

#[test]
fn test_scan_scope() -> Result<(), anyhow::Error> {
    let source = "INSERT OR REPLACE  INTO";
    let engine = sqlite_engine::create()?;
    let mut scanner = Scanner::create(source, 0, engine.scanning_rules)?;

    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::r#INSERT, offset: 0, len: 6, value: Some("INSERT".into()) },
            trailing_trivia: Some(vec![
                ScanEvent { kind: syntax_kind::r#SPACE, offset: 6, len: 1, value: Some(" ".into()) }
            ]),
        };
        assert_eq!(Some(expect_token), scanner.shift());
        break 'scanning;
    }
    let mut scope = 'save_scope: {
        break 'save_scope scanner.save_scope();
    };
    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::r#OR, offset: 7, len: 2, value: Some("OR".into()) },
            trailing_trivia: Some(vec![
                ScanEvent { kind: syntax_kind::r#SPACE, offset: 9, len: 1, value: Some(" ".into()) }
            ]),
        };
        assert_eq!(Some(expect_token), scope.cache_lookahead(scanner.shift()));
        break 'scanning;
    }
    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::r#REPLACE, offset: 10, len: 7, value: Some("REPLACE".into()) },
            trailing_trivia: Some(vec![
                ScanEvent { kind: syntax_kind::r#SPACE, offset: 17, len: 2, value: Some("  ".into()) }
            ]),
        };
        assert_eq!(Some(expect_token), scope.cache_lookahead(scanner.shift()));
        break 'scanning;
    }
    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::r#INTO, offset: 19, len: 4, value: Some("INTO".into()) },
            trailing_trivia: None,
        };
        assert_eq!(Some(expect_token), scope.cache_lookahead(scanner.shift()));
        break 'scanning;
    }
    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::r#EOF, offset: 23, len: 0, value: None },
            trailing_trivia: None,
        };
        assert_eq!(Some(expect_token), scope.cache_lookahead(scanner.shift()));
        break 'scanning;
    }
    'scanning: {
        assert_eq!(None, scope.cache_lookahead(scanner.shift()));
        break 'scanning;
    }
    'restore_scope: {
        scanner.restore_scope(scope);
        break 'restore_scope;
    }
    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::r#OR, offset: 7, len: 2, value: Some("OR".into()) },
            trailing_trivia: Some(vec![
                ScanEvent { kind: syntax_kind::r#SPACE, offset: 9, len: 1, value: Some(" ".into()) }
            ]),
        };
        assert_eq!(Some(expect_token), scanner.shift());
        break 'scanning;
    }
    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::r#REPLACE, offset: 10, len: 7, value: Some("REPLACE".into()) },
            trailing_trivia: Some(vec![
                ScanEvent { kind: syntax_kind::r#SPACE, offset: 17, len: 2, value: Some("  ".into()) }
            ]),
        };
        assert_eq!(Some(expect_token), scanner.shift());
        break 'scanning;
    }
    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::r#INTO, offset: 19, len: 4, value: Some("INTO".into()) },
            trailing_trivia: None,
        };
        assert_eq!(Some(expect_token), scanner.shift());
        break 'scanning;
    }
    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::r#EOF, offset: 23, len: 0, value: None },
            trailing_trivia: None,
        };
        assert_eq!(Some(expect_token), scanner.shift());
        break 'scanning;
    }
    'scanning: {
        assert_eq!(None, scanner.shift());
        break 'scanning;
    }

    Ok(())
}