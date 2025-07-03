#![cfg(not(engine_ungenerated))]

use engine_core::scanner_engine::ScanEvent;
use scanner_core::{Scanner, ScannerAccess, Token};
use sqlite_engine::syntax_kind;

mod scanner_tests_members {
    mod scan_dispatcher_tests;
    mod prefetch_tests;
}

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
fn test_match_incorrect_identifier() -> Result<(), anyhow::Error> {
    let source = "あ";
    let engine = sqlite_engine::create()?.scanning_rules;
    let scanner = Scanner::create(source, 0, engine)?;

    let expect_token = Token {
        leading_trivia: None,
        main: ScanEvent { kind: syntax_kind::r#ILLEGAL, offset: 0, len: 3, value: Some("あ".into()) },
        trailing_trivia: None,
    };

    assert_eq!(Some(&expect_token), scanner.lookahead());
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
fn test_has_invalid_token() -> Result<(), anyhow::Error> {
    let source = "SELECT 1 } FROM foo";
    let engine = sqlite_engine::create()?;
    let mut scanner = Scanner::create(source, 0, engine.scanning_rules)?;

    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::r#SELECT, offset: 0, len: 6, value: Some("SELECT".into()) },
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
            main: ScanEvent { kind: syntax_kind::r#INTEGER, offset: 7, len: 1, value: Some("1".into()) },
            trailing_trivia: Some(vec![
                ScanEvent { kind: syntax_kind::r#SPACE, offset: 8, len: 1, value: Some(" ".into()) }
            ]),
        };
        assert_eq!(Some(expect_token), scanner.shift());
        break 'scanning;
    }
    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::ILLEGAL, offset: 9, len: 1, value: Some("}".into()) },
            trailing_trivia: Some(vec![
                ScanEvent { kind: syntax_kind::r#SPACE, offset: 10, len: 1, value: Some(" ".into()) }
            ]),
        };
        assert_eq!(Some(expect_token), scanner.shift());
        break 'scanning;
    }
    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::FROM, offset: 11, len: 4, value: Some("FROM".into()) },
            trailing_trivia: Some(vec![
                ScanEvent { kind: syntax_kind::r#SPACE, offset: 15, len: 1, value: Some(" ".into()) }
            ]),
        };
        assert_eq!(Some(expect_token), scanner.shift());
        break 'scanning;
    }
    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::ID, offset: 16, len: 3, value: Some("foo".into()) },
            trailing_trivia: None,
        };
        assert_eq!(Some(expect_token), scanner.shift());
        break 'scanning;
    }
    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::EOF, offset: 19, len: 0, value: None },
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
fn test_scan_semicollonless() -> Result<(), anyhow::Error> {
    let source = "select 1 a";
    let engine = sqlite_engine::create()?;
    let mut scanner = Scanner::create(source, 0, engine.scanning_rules)?;

    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::r#SELECT, offset: 0, len: 6, value: Some("select".into()) },
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
            main: ScanEvent { kind: syntax_kind::r#INTEGER, offset: 7, len: 1, value: Some("1".into()) },
            trailing_trivia: Some(vec![
                ScanEvent { kind: syntax_kind::r#SPACE, offset: 8, len: 1, value: Some(" ".into()) }
            ]),
        };
        assert_eq!(Some(expect_token), scanner.shift());
        break 'scanning;
    }
    'scanning: {
        let expect_token = Token {
            leading_trivia: None,
            main: ScanEvent { kind: syntax_kind::r#ID, offset: 9, len: 1, value: Some("a".into()) },
            trailing_trivia: None,
        };
        assert_eq!(Some(expect_token), scanner.shift());
        break 'scanning;
    }

    Ok(())
}