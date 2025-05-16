use parser_core::event_dispatcher::{ParseEventDispatcher, ParseEvent};
use engine_core::Engine;

#[cfg(test)]
mod default_scanner_engine_tests {
    use super::*;

    #[test]
    fn test_next_event() -> Result<(), anyhow::Error> {
        let engine = Engine::default().parsing_rules;
        let eof_kind = engine.eof();
        let mut dispatcher = ParseEventDispatcher::new(0, engine);

        let expect_event = ParseEvent::Shift { kind: eof_kind, current_state: 0, next_state: 0, edit_state: 0 };
        assert_eq!(Ok(expect_event), dispatcher.next(Some(eof_kind)));
        assert_eq!(true, dispatcher.state_values().is_empty());
        Ok(())
    }
}

#[cfg(test)]
#[cfg(not(engine_ungenerated))]
mod scanner_engine_tests {
    use parser_core::ParseError;
    use sqlite_engine::syntax_kind;
    use super::*;

    #[test]
    fn test_parse_empty_source() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?.parsing_rules;
        let mut dispatcher = ParseEventDispatcher::new(0, engine);

        let expect_event = ParseEvent::Shift { kind: syntax_kind::r#EOF, current_state: 0, next_state: 0, edit_state: 0 };
        assert_eq!(Ok(expect_event), dispatcher.next(Some(syntax_kind::r#EOF)));
        assert_eq!(true, dispatcher.state_values().is_empty());
        Ok(())
    }

    #[test]
    fn test_parse_shift() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?.parsing_rules;
        let mut dispatcher = ParseEventDispatcher::new(0, engine);

        'next_state: {
            let expected_event = ParseEvent::Shift{ kind: syntax_kind::r#SELECT, current_state: 0, next_state: 18, edit_state: 0 };
            assert_eq!(Ok(expected_event), dispatcher.next(Some(syntax_kind::r#SELECT)));
            assert_eq!(vec![18, 0], dispatcher.state_values());
            break 'next_state;
        }
        'next_state: {
            let expected_event = ParseEvent::Shift{ kind: syntax_kind::r#DISTINCT, current_state: 18, next_state: 70, edit_state: 18 };
            assert_eq!(Ok(expected_event), dispatcher.next(Some(syntax_kind::r#DISTINCT)));
            assert_eq!(vec![70, 18, 0], dispatcher.state_values());
            break 'next_state;
        }
        Ok(())
    }

    #[test]
    fn test_parse_reduce() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?.parsing_rules;
        let mut dispatcher = ParseEventDispatcher::new(0, engine);

        'next_state: {
            let expected_event = ParseEvent::Shift{ kind: syntax_kind::r#SELECT, current_state: 0, next_state: 18, edit_state: 0 };
            assert_eq!(Ok(expected_event), dispatcher.next(Some(syntax_kind::r#SELECT)));
            assert_eq!(vec![18, 0], dispatcher.state_values());
            break 'next_state;
        }
        'next_state: {
            let expected_event = ParseEvent::Reduce{ kind: syntax_kind::r#distinct, pop_count: 0, current_state: 18, next_state: 71, edit_state: 18 };
            assert_eq!(Ok(expected_event), dispatcher.next(Some(syntax_kind::r#INTEGER)));
            assert_eq!(vec![71, 18, 0], dispatcher.state_values());
            break 'next_state;
        }
        'next_state: {
            let expected_event = ParseEvent::Reduce{ kind: syntax_kind::r#sclp, pop_count: 0, current_state: 71, next_state: 144, edit_state: 71 };
            assert_eq!(Ok(expected_event), dispatcher.next(Some(syntax_kind::r#INTEGER)));
            assert_eq!(vec![144, 71, 18, 0], dispatcher.state_values());
            break 'next_state;
        }
        'next_state: {
            let expected_event = ParseEvent::Reduce{ kind: syntax_kind::r#scanpt, pop_count: 0, current_state: 144, next_state: 238, edit_state: 144 };
            assert_eq!(Ok(expected_event), dispatcher.next(Some(syntax_kind::r#INTEGER)));
            assert_eq!(vec![238, 144, 71, 18, 0], dispatcher.state_values());
            break 'next_state;
        }
        'next_state: {
            let expected_event = ParseEvent::Shift{ kind: syntax_kind::r#INTEGER, current_state: 238, next_state: 122, edit_state: 238 };
            assert_eq!(Ok(expected_event), dispatcher.next(Some(syntax_kind::r#INTEGER)));
            assert_eq!(vec![122, 238, 144, 71, 18, 0], dispatcher.state_values());
            break 'next_state;
        }
        'next_state: {
            let expected_event = ParseEvent::Reduce{ kind: syntax_kind::r#term, pop_count: 1, current_state: 122, next_state: 128, edit_state: 238 };
            assert_eq!(Ok(expected_event), dispatcher.next(Some(syntax_kind::r#SEMI)));
            assert_eq!(vec![128, 238, 144, 71, 18, 0], dispatcher.state_values());
            break 'next_state;
        }

        Ok(())
    }

    #[test]
    fn test_parse_accept() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?.parsing_rules;
        let mut dispatcher = ParseEventDispatcher::new(22, engine);

        'next_state: {
            let expected_event = ParseEvent::Shift{ kind: syntax_kind::r#EOF, current_state: 22, next_state: 74, edit_state: 22 };
            assert_eq!(Ok(expected_event), dispatcher.next(Some(syntax_kind::r#EOF)));
            assert_eq!(vec![74, 22], dispatcher.state_values());
            break 'next_state;
        }
        'next_state: {
            let expected_event = ParseEvent::Accept{ kind: syntax_kind::r#input, last_state: 74, edit_state: 0 };
            assert_eq!(Ok(expected_event), dispatcher.next(None));
            assert_eq!(Vec::<usize>::new(), dispatcher.state_values());
            break 'next_state;
        }

        Ok(())
    }

    #[test]
    fn test_no_more_state() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?.parsing_rules;
        let mut dispatcher = ParseEventDispatcher::new(74, engine);
            
        'next_state: {
            let expected_event = ParseEvent::Accept{ kind: syntax_kind::r#input, last_state: 74, edit_state: 0 };
            assert_eq!(Ok(expected_event), dispatcher.next(None));
            assert_eq!(Vec::<usize>::new(), dispatcher.state_values());
            break 'next_state;
        }
        'next_state: {
            assert_eq!(Err(ParseError::NoMoreState { context: "Shift".into() }), dispatcher.next(None));
            assert_eq!(Vec::<usize>::new(), dispatcher.state_values());
            break 'next_state;
        }

        Ok(())
    }

    #[test]
    fn test_not_acceptable_state() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?.parsing_rules;
        let mut dispatcher = ParseEventDispatcher::new(0, engine);
            
        'next_state: {
            assert_eq!(Err(ParseError::NotAccept), dispatcher.next(None));
            assert_eq!(vec![0], dispatcher.state_values());
            break 'next_state;
        }

        Ok(())
    }

    mod broken_table {
        use engine_core::parser_engine::{ParsingRuleSet, Transition};

        use super::*;

        #[test]
        fn test_goto_failed() -> Result<(), anyhow::Error> {
            let engine = ParsingRuleSet::new(
                next_lookahead_translation,
                next_goto_translation,
                get_accept_transition,
                lookup_symbol,
                0,
            );
            sqlite_engine::create()?.parsing_rules;
            let mut dispatcher = ParseEventDispatcher::new(1, engine);

            'next_state: {
                assert_eq!(Err(ParseError::NoGotoCandidate { state: 1, lhs: "EOF".into() }), dispatcher.next(Some(syntax_kind::r#SEMI)));
                assert_eq!(vec![1], dispatcher.state_values());
                break 'next_state;
            }

            Ok(())
        }

        static DUMMY_LA_TRANSITION: Transition = Transition::Reduce { pop_count: 0, lhs: 1 };
        fn next_lookahead_translation(_kind: u32, _state: usize) -> Option<&'static Transition> {
            Some(&DUMMY_LA_TRANSITION)
        }
        fn next_goto_translation(_kind: u32, _state: usize) -> Option<&'static usize> {
            None
        }
        fn get_accept_transition() -> Option<&'static Transition> {
            None
        }
        fn lookup_symbol(_id: u32) -> &'static engine_core::SyntaxKind {
            &syntax_kind::r#EOF
        }
    }

// [DEBUG] Shift/kind: SELECT, push: [18, 0]
// [DEBUG] Reduce/kind: distinct, pop(0)&push: [72, 18, 0]
// [DEBUG] Reduce/kind: sclp, pop(0)&push: [145, 72, 18, 0]
// [DEBUG] Reduce/kind: scanpt, pop(0)&push: [239, 145, 72, 18, 0]
// [DEBUG] Shift/kind: INTEGER, push: [123, 239, 145, 72, 18, 0]
// [DEBUG] Reduce/kind: term, pop(1)&push: [129, 239, 145, 72, 18, 0]
// [DEBUG] Reduce/kind: expr, pop(1)&push: [362, 239, 145, 72, 18, 0]
// [DEBUG] Reduce/kind: scanpt, pop(0)&push: [468, 362, 239, 145, 72, 18, 0]
// [DEBUG] Reduce/kind: as, pop(0)&push: [580, 468, 362, 239, 145, 72, 18, 0]
// [DEBUG] Reduce/kind: selcollist, pop(5)&push: [146, 72, 18, 0]
// [DEBUG] Reduce/kind: from, pop(0)&push: [242, 146, 72, 18, 0]
// [DEBUG] Reduce/kind: where_opt, pop(0)&push: [367, 242, 146, 72, 18, 0]
// [DEBUG] Reduce/kind: groupby_opt, pop(0)&push: [478, 367, 242, 146, 72, 18, 0]
// [DEBUG] Reduce/kind: having_opt, pop(0)&push: [589, 478, 367, 242, 146, 72, 18, 0]
// [DEBUG] Reduce/kind: orderby_opt, pop(0)&push: [673, 589, 478, 367, 242, 146, 72, 18, 0]
// [DEBUG] Reduce/kind: limit_opt, pop(0)&push: [747, 673, 589, 478, 367, 242, 146, 72, 18, 0]
// [DEBUG] Reduce/kind: oneselect, pop(9)&push: [31, 0]
// [DEBUG] Reduce/kind: selectnowith, pop(1)&push: [33, 0]
// [DEBUG] Reduce/kind: select, pop(1)&push: [32, 0]
// [DEBUG] Reduce/kind: cmd, pop(1)&push: [21, 0]
// [DEBUG] Reduce/kind: cmdx, pop(1)&push: [23, 0]
}
