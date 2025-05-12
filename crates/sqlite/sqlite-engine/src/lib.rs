

#[cfg(engine_generated)]
mod generated {    
    include!("_generated/symbol_set.rs");
    include!("_generated/scan_rule.rs");

    pub fn get_lexme_pattern(prefix: char) -> Option<&'static [engine_core::scanner_engine::ScanPattern]> {
        scan_rule_map::LEXME_SCAN_RULE.get(&prefix).cloned()
    }

    pub fn get_regex_pattern(_id: u32) -> Option<&'static engine_core::scanner_engine::ScanPattern> {
        todo!("未実装: get_regex_pattern")
    }

    pub fn get_symbol(symbol_id: u32) -> &'static engine_core::SyntaxKind {
        syntax_map::SYNTAX_KIND_MAP.get(&symbol_id).cloned().unwrap_or(&syntax_kind::r#ILLEGAL)
    }
}

#[cfg(engine_generated)]
pub fn create() -> Result<engine_core::Engine, engine_core::EngineError> {
    use generated::syntax_kind;

    Ok(engine_core::Engine {
        symbol_rules: Default::default(),
        scanning_rules: engine_core::scanner_engine::ScanningRuleSet::new(
            generated::get_lexme_pattern,
            generated::get_regex_pattern,
            generated::get_symbol,
            syntax_kind::r#EOF.id,
        ),
        parsing_rules: Default::default(),
    })
}
#[cfg(not(engine_generated))]
pub fn create() -> Result<engine_core::Engine, engine_core::EngineError> {
    Ok(engine_core::Engine::default())
}

#[cfg(test)]
mod default_scanner_engine_tests {
    use engine_core::Engine;
    use scanner_core::dispatch::ScanEventDispatcher;

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

        assert_eq!(None, dispatcher.next_regex());
        Ok(())
    }
}

#[cfg(test)]
#[cfg(engine_generated)]
mod scan_by_lexme_tests {
    use engine_core::{scanner_engine::ScanEvent, SyntaxKind};
    use scanner_core::dispatch::ScanEventDispatcher;
    use crate::generated::syntax_kind;

    #[test]
    fn test_accepted() -> Result<(), anyhow::Error> {
        let source = "FROM foo";
        let engine = super::create()?;
        let mut dispatcher = ScanEventDispatcher::new(source, 0, engine.scanning_rules);
        
        let expect_event = ScanEvent {
            offset:0, len:4, value: Some("FROM".into()), kind: syntax_kind::r#FROM.clone()
        };
        assert_eq!(Some(expect_event), dispatcher.next_lexme());

        assert_eq!(None, dispatcher.next_lexme());
        assert_eq!(None, dispatcher.next_lexme());
        Ok(())
    }

    #[test]
    fn test_rejected() -> Result<(), anyhow::Error> {
        let source = "qwerty";
        let engine = super::create()?.scanning_rules;
        let mut dispatcher = ScanEventDispatcher::new(source, 0, engine);
        assert_eq!(None, dispatcher.next_lexme());
        Ok(())
    }

    #[test]
    fn test_empty_source() -> Result<(), anyhow::Error> {
        let source = "";
        let engine = super::create()?.scanning_rules;
        let mut dispatcher = ScanEventDispatcher::new(source, 0, engine);

        let expect_event = ScanEvent {
            offset:0, len:0, value: None, kind: syntax_kind::r#EOF.clone()
        };
        assert_eq!(Some(expect_event), dispatcher.next_lexme());
        assert_eq!(None, dispatcher.next_lexme());
        Ok(())
    }

    #[test]
    fn test_overflow_offset() -> Result<(), anyhow::Error> {
        let source = "";
        let engine = super::create()?.scanning_rules;
        let mut dispatcher = ScanEventDispatcher::new(source, 1, engine);

        assert_eq!(None, dispatcher.next_lexme());
        Ok(())
    }

    #[test]
    fn test_short_match_all() -> Result<(), anyhow::Error> {
        let source = "INSERTORREPLACEINTO";
        let engine = super::create()?.scanning_rules;
        let mut dispatcher = ScanEventDispatcher::new(source, 0, engine);

        let expect_event_1 = ScanEvent {
            offset:0, len:6, value: Some("INSERT".into()), kind: syntax_kind::r#INSERT.clone()
        };
        let expect_event_2 = ScanEvent {
            offset:6, len:2, value: Some("OR".into()), kind: syntax_kind::r#OR.clone()
        };
        let expect_event_3 = ScanEvent {
            offset:8, len:7, value: Some("REPLACE".into()), kind: syntax_kind::r#REPLACE.clone()
        };
        let expect_event_4 = ScanEvent {
            offset:15, len:4, value: Some("INTO".into()), kind: syntax_kind::r#INTO.clone()
        };
        let expect_event_5 = ScanEvent {
            offset:19, len:0, value: None, kind: syntax_kind::r#EOF.clone()
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
mod scan_by_regex_tests {
    use scanner_core::dispatch::ScanEventDispatcher;

    #[test]
    fn test_accepted() -> Result<(), anyhow::Error> {
        todo!()
    }

    #[test]
    fn test_rejected() -> Result<(), anyhow::Error> {
        todo!()
    }

    #[test]
    fn test_empty_source() -> Result<(), anyhow::Error> {
        todo!()
    }

    #[test]
    fn test_overflow_offset() -> Result<(), anyhow::Error> {
        let source = "";
        let engine = super::create()?.scanning_rules;
        let mut dispatcher = ScanEventDispatcher::new(source, 1, engine);

        assert_eq!(None, dispatcher.next_regex());
        Ok(())
    }

    #[test]
    fn test_scan_all() -> Result<(), anyhow::Error> {
        todo!()
    }
}

#[cfg(test)]
mod scan_greedy_tests {
    #[test]
    fn test_accepted() -> Result<(), anyhow::Error> {
        todo!()
    }

    #[test]
    fn test_rejected() -> Result<(), anyhow::Error> {
        todo!()
    }

    #[test]
    fn test_empty_source() -> Result<(), anyhow::Error> {
        todo!()
    }

    #[test]
    fn test_overflow_offset() -> Result<(), anyhow::Error> {
        todo!()
    }

    #[test]
    fn test_scan_all() -> Result<(), anyhow::Error> {
        todo!()
    }
}
