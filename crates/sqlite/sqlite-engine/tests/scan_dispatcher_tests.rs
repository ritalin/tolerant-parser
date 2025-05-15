use scanner_core::dispatch::ScanEventDispatcher;
use engine_core::scanner_engine::{AcceptableRegexSet, ScanEvent};

#[cfg(test)]
mod default_scanner_engine_tests {
    use engine_core::{scanner_engine::AcceptableRegexSet, Engine};
    use super::*;

    #[test]
    fn test_by_lexme() -> Result<(), anyhow::Error> {
        let source = "FROM";
        let engine = Engine::default().scanning_rules;
        let mut dispatcher = ScanEventDispatcher::new(source, 0, engine);

        assert_eq!(None, dispatcher.next_lexme());
        Ok(())
    }

    #[test]
    fn test_by_regex() -> Result<(), anyhow::Error> {
        let source = "qwerty";
        let engine = Engine::default().scanning_rules;
        let mut dispatcher = ScanEventDispatcher::new(source, 0, engine);

        assert_eq!(None, dispatcher.next_regex(&AcceptableRegexSet::Main));
        Ok(())
    }
}

#[cfg(test)]
#[cfg(not(engine_ungenerated))]
mod scan_by_lexme_tests {
    use sqlite_engine::syntax_kind;
    use super::*;

    #[test]
    fn test_accepted() -> Result<(), anyhow::Error> {
        let source = "FROM foo";
        let engine = sqlite_engine::create()?;
        let mut dispatcher = ScanEventDispatcher::new(source, 0, engine.scanning_rules);
        
        let expect_event = ScanEvent {
            offset:0, len:4, value: Some("FROM".into()), kind: syntax_kind::r#FROM
        };
        assert_eq!(Some(expect_event), dispatcher.next_lexme());

        assert_eq!(None, dispatcher.next_lexme());
        assert_eq!(None, dispatcher.next_lexme());
        Ok(())
    }

    #[test]
    fn test_rejected() -> Result<(), anyhow::Error> {
        let source = "qwerty";
        let engine = sqlite_engine::create()?.scanning_rules;
        let mut dispatcher = ScanEventDispatcher::new(source, 0, engine);
        assert_eq!(None, dispatcher.next_lexme());
        Ok(())
    }

    #[test]
    fn test_empty_source() -> Result<(), anyhow::Error> {
        let source = "";
        let engine = sqlite_engine::create()?.scanning_rules;
        let mut dispatcher = ScanEventDispatcher::new(source, 0, engine);

        let expect_event = ScanEvent {
            offset:0, len:0, value: None, kind: syntax_kind::r#EOF
        };
        assert_eq!(Some(expect_event), dispatcher.next_lexme());
        assert_eq!(None, dispatcher.next_lexme());
        Ok(())
    }

    #[test]
    fn test_overflow_offset() -> Result<(), anyhow::Error> {
        let source = "";
        let engine = sqlite_engine::create()?.scanning_rules;
        let mut dispatcher = ScanEventDispatcher::new(source, 1, engine);

        assert_eq!(None, dispatcher.next_lexme());
        Ok(())
    }

    #[test]
    fn test_short_match_all() -> Result<(), anyhow::Error> {
        let source = "INSERTORREPLACEINTO";
        let engine = sqlite_engine::create()?.scanning_rules;
        let mut dispatcher = ScanEventDispatcher::new(source, 0, engine);

        let expect_event_1 = ScanEvent {
            offset:0, len:6, value: Some("INSERT".into()), kind: syntax_kind::r#INSERT
        };
        let expect_event_2 = ScanEvent {
            offset:6, len:2, value: Some("OR".into()), kind: syntax_kind::r#OR
        };
        let expect_event_3 = ScanEvent {
            offset:8, len:7, value: Some("REPLACE".into()), kind: syntax_kind::r#REPLACE
        };
        let expect_event_4 = ScanEvent {
            offset:15, len:4, value: Some("INTO".into()), kind: syntax_kind::r#INTO
        };
        let expect_event_5 = ScanEvent {
            offset:19, len:0, value: None, kind: syntax_kind::r#EOF
        };

        assert_eq!(Some(expect_event_1), dispatcher.next_lexme());
        assert_eq!(Some(expect_event_2), dispatcher.next_lexme());
        assert_eq!(Some(expect_event_3), dispatcher.next_lexme());
        assert_eq!(Some(expect_event_4), dispatcher.next_lexme());
        assert_eq!(Some(expect_event_5), dispatcher.next_lexme());
        assert_eq!(None, dispatcher.next_lexme());
        Ok(())
    }
}

#[cfg(test)]
#[cfg(not(engine_ungenerated))]
mod scan_by_regex_tests {
    use sqlite_engine::syntax_kind;
    use super::*;

    #[test]
    fn test_accepted() -> Result<(), anyhow::Error> {
        let source = "FROM foo";
        let engine = sqlite_engine::create()?;
        let mut dispatcher = ScanEventDispatcher::new(source, 0, engine.scanning_rules);

        let expect_event = ScanEvent {
            offset:0, len:4, value: Some("FROM".into()), kind: syntax_kind::r#ID
        };
        assert_eq!(Some(expect_event), dispatcher.next_regex(&AcceptableRegexSet::Main));
        Ok(())
    }

    #[test]
    fn test_rejected() -> Result<(), anyhow::Error> {
        let source = "FROM foo";
        let engine = sqlite_engine::create()?;
        let mut dispatcher = ScanEventDispatcher::new(source, 0, engine.scanning_rules);
        assert_eq!(None, dispatcher.next_regex(&AcceptableRegexSet::Leading));
        Ok(())
    }

    #[test]
    fn test_empty_source() -> Result<(), anyhow::Error> {
        let source = "";
        let engine = sqlite_engine::create()?.scanning_rules;
        let mut dispatcher = ScanEventDispatcher::new(source, 0, engine);

        let expect_event = ScanEvent {
            offset:0, len:0, value: None, kind: syntax_kind::r#EOF
        };
        assert_eq!(Some(expect_event), dispatcher.next_regex(&AcceptableRegexSet::Main));
        assert_eq!(None, dispatcher.next_regex(&AcceptableRegexSet::Main));
        Ok(())
    }

    #[test]
    fn test_empty_source_for_main_token() -> Result<(), anyhow::Error> {
        let source = "";
        let engine = sqlite_engine::create()?.scanning_rules;
        let mut dispatcher = ScanEventDispatcher::new(source, 0, engine);

        let expect_event = ScanEvent {
            offset:0, len:0, value: None, kind: syntax_kind::r#EOF
        };
        assert_eq!(Some(expect_event), dispatcher.next_regex(&AcceptableRegexSet::Main));
        assert_eq!(None, dispatcher.next_regex(&AcceptableRegexSet::Main));
        Ok(())
    }

    #[test]
    fn test_empty_source_trivia() -> Result<(), anyhow::Error> {
        let source = "";
        let engine = sqlite_engine::create()?.scanning_rules;
        let mut dispatcher = ScanEventDispatcher::new(source, 0, engine);

        assert_eq!(None, dispatcher.next_regex(&AcceptableRegexSet::Leading));
        assert_eq!(None, dispatcher.next_regex(&AcceptableRegexSet::Trailing));
        Ok(())
    }

    #[test]
    fn test_overflow_offset() -> Result<(), anyhow::Error> {
        let source = "";
        let engine = sqlite_engine::create()?.scanning_rules;
        let mut dispatcher = ScanEventDispatcher::new(source, 1, engine);

        assert_eq!(None, dispatcher.next_regex(&AcceptableRegexSet::Leading));
        assert_eq!(None, dispatcher.next_regex(&AcceptableRegexSet::Main));
        assert_eq!(None, dispatcher.next_regex(&AcceptableRegexSet::Trailing));
        Ok(())
    }

    #[test]
    fn test_match_all() -> Result<(), anyhow::Error> {
        let source = "INSERTORREPLACEINTO";
        let engine = sqlite_engine::create()?.scanning_rules;
        let mut dispatcher = ScanEventDispatcher::new(source, 0, engine);

        let expect_event_1 = ScanEvent {
            offset:0, len:19, value: Some("INSERTORREPLACEINTO".into()), kind: syntax_kind::r#ID
        };
        let expect_event_2 = ScanEvent {
            offset:19, len:0, value: None, kind: syntax_kind::r#EOF
        };
        assert_eq!(Some(expect_event_1), dispatcher.next_regex(&AcceptableRegexSet::Main));
        assert_eq!(Some(expect_event_2), dispatcher.next_regex(&AcceptableRegexSet::Main));
        assert_eq!(None, dispatcher.next_regex(&AcceptableRegexSet::Main));
        Ok(())
    }
}

#[cfg(test)]
#[cfg(not(engine_ungenerated))]
mod scan_greedy_tests {
    use engine_core::scanner_engine::{AcceptableRegexSet, ScanEvent};
    use sqlite_engine::syntax_kind;
    use super::*;

    #[test]
    fn test_rejected() -> Result<(), anyhow::Error> {
        let source = "$$";
        let engine = sqlite_engine::create()?.scanning_rules;
        let mut dispatcher = ScanEventDispatcher::new(source, 0, engine);
        assert_eq!(None, dispatcher.next(&AcceptableRegexSet::Leading));
        Ok(())
    }

    #[test]
    fn test_empty_source() -> Result<(), anyhow::Error> {
        let source = "";
        let engine = sqlite_engine::create()?.scanning_rules;
        let mut dispatcher = ScanEventDispatcher::new(source, 0, engine);

        let expect_event = ScanEvent {
            offset:0, len:0, value: None, kind: syntax_kind::r#EOF
        };
        assert_eq!(Some(expect_event), dispatcher.next(&AcceptableRegexSet::Main));
        assert_eq!(None, dispatcher.next(&AcceptableRegexSet::Main));
        Ok(())
    }

    #[test]
    fn test_empty_source_trivia() -> Result<(), anyhow::Error> {
        let source = "";
        let engine = sqlite_engine::create()?.scanning_rules;
        let mut dispatcher = ScanEventDispatcher::new(source, 0, engine);

        assert_eq!(None, dispatcher.next(&AcceptableRegexSet::Leading));
        assert_eq!(None, dispatcher.next(&AcceptableRegexSet::Trailing));
        Ok(())
    }

    #[test]
    fn test_overflow_offset() -> Result<(), anyhow::Error> {
        let source = "";
        let engine = sqlite_engine::create()?.scanning_rules;
        let mut dispatcher = ScanEventDispatcher::new(source, 1, engine);

        assert_eq!(None, dispatcher.next(&AcceptableRegexSet::Main));
        Ok(())
    }

    #[test]
    fn test_match_all() -> Result<(), anyhow::Error> {
        let source = "INSERTORREPLACEINTO";
        let engine = sqlite_engine::create()?.scanning_rules;
        let mut dispatcher = ScanEventDispatcher::new(source, 0, engine);

        let expect_event_1 = ScanEvent {
            offset:0, len:19, value: Some("INSERTORREPLACEINTO".into()), kind: syntax_kind::r#ID
        };
        let expect_event_2 = ScanEvent {
            offset:19, len:0, value: None, kind: syntax_kind::r#EOF
        };
        assert_eq!(Some(expect_event_1), dispatcher.next(&AcceptableRegexSet::Main));
        assert_eq!(Some(expect_event_2), dispatcher.next(&AcceptableRegexSet::Main));
        assert_eq!(None, dispatcher.next(&AcceptableRegexSet::Main));
        Ok(())
    }
}
