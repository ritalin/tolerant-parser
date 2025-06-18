use parser_core::event_dispatcher::{ParseEventDispatcher, ParseEvent};
use engine_core::Engine;

#[cfg(test)]
mod default_scanner_engine_tests {
    use parser_core::ParseMode;

    use super::*;

    #[test]
    fn test_next_event() -> Result<(), anyhow::Error> {
        let engine = Engine::default().parsing_rules;
        let eof_kind = engine.full_emit_config().to_symbol;
        let mut dispatcher = ParseEventDispatcher::new(0, ParseMode::ByStatement, engine);

        let expect_event = ParseEvent::Shift { kind: eof_kind, current_state: 0, next_state: 0, edit_state: 0 };
        assert_eq!(Ok(expect_event), dispatcher.next(Some(eof_kind)));
        assert_eq!(vec![0], dispatcher.state_values());
        Ok(())
    }
}

#[cfg(test)]
#[cfg(not(engine_ungenerated))]
mod dispatcher_tests {
    use parser_core::{event_dispatcher::ParseEventError, ParseMode};
    use sqlite_engine::syntax_kind;
    use super::*;

    #[test]
    fn test_parse_empty_source() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?.parsing_rules;
        let mut dispatcher = ParseEventDispatcher::new(0, ParseMode::ByStatement, engine);

        let expect_event = ParseEvent::Shift { kind: syntax_kind::r#EOF, current_state: 0, next_state: 0, edit_state: 0 };
        assert_eq!(Ok(expect_event), dispatcher.next(Some(syntax_kind::r#EOF)));
        assert_eq!(vec![0], dispatcher.state_values());
        Ok(())
    }

    #[test]
    fn test_parse_shift() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?.parsing_rules;
        let mut dispatcher = ParseEventDispatcher::new(0, ParseMode::ByStatement, engine);

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
        let mut dispatcher = ParseEventDispatcher::new(0, ParseMode::ByStatement, engine);

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
        let mut dispatcher = ParseEventDispatcher::new(22, ParseMode::ByStatement, engine);

        'next_state: {
            let expected_event = ParseEvent::PatchEmit{ kind: syntax_kind::r#ecmd, edit_state: 22 };
            assert_eq!(Ok(expected_event), dispatcher.next(Some(syntax_kind::r#EOF)));
            assert_eq!(vec![74, 22], dispatcher.state_values());
            break 'next_state;
        }
        'next_state: {
            let expected_event = ParseEvent::Shift{ kind: syntax_kind::r#EOF, current_state: 22, next_state: 74, edit_state: 22 };
            assert_eq!(Ok(expected_event), dispatcher.next(Some(syntax_kind::r#EOF)));
            assert_eq!(vec![74, 22], dispatcher.state_values());
            break 'next_state;
        }
        'next_state: {
            let expected_event = ParseEvent::Emit{ kind: syntax_kind::r#ecmd, edit_state: 22 };
            assert_eq!(Ok(expected_event), dispatcher.next(None));
            assert_eq!(vec![74, 22], dispatcher.state_values());
            break 'next_state;
        }
        'next_state: {
            let expected_event = ParseEvent::Accept{ kind: syntax_kind::r#input, last_state: 22, edit_state: 22 };
            assert_eq!(Ok(expected_event), dispatcher.next(None));
            assert_eq!(vec![74, 22], dispatcher.state_values());
            break 'next_state;
        }

        Ok(())
    }

    #[test]
    fn test_no_more_state() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?.parsing_rules;
        let mut dispatcher = ParseEventDispatcher::new(74, ParseMode::ByStatement, engine);
            
        'next_state: {
            let expected_event = ParseEvent::Accept{ kind: syntax_kind::r#input, last_state: 74, edit_state: 0 };
            assert_eq!(Ok(expected_event), dispatcher.next(None));
            assert_eq!(Vec::<usize>::new(), dispatcher.state_values());
            break 'next_state;
        }
        'next_state: {
            assert_eq!(Err(ParseEventError::NoMoreState { context: "Shift".into() }), dispatcher.next(None));
            assert_eq!(Vec::<usize>::new(), dispatcher.state_values());
            break 'next_state;
        }

        Ok(())
    }

    #[test]
    fn test_not_acceptable_state() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?.parsing_rules;
        let mut dispatcher = ParseEventDispatcher::new(0, ParseMode::ByStatement, engine);
            
        'next_state: {
            assert_eq!(Err(ParseEventError::NotAccept), dispatcher.next(None));
            assert_eq!(vec![0], dispatcher.state_values());
            break 'next_state;
        }

        Ok(())
    }

    mod broken_table {
        use engine_core::{parser_engine::{ParsingRuleSetBuilder, Transition}, EmitRegin};

        use super::*;

        #[test]
        fn test_goto_failed() -> Result<(), anyhow::Error> {
            let engine = ParsingRuleSetBuilder::default()
                .lookahead_translation(next_lookahead_translation)
                .goto_translation(|_kind, _state| None)
                .accept_transition(|| None)
                .symbol_lookup(|_| &syntax_kind::r#EOF)
                .alternative_symbol_lookup(|_p, _c| None)
                .candidate_symbols(|_| vec![])
                .full_emit_region(EmitRegin::default())
                .statement_emit_region(EmitRegin::default())
                .invalid_statement_emit_region(EmitRegin::default())
                .build()?
            ;

            sqlite_engine::create()?.parsing_rules;
            let mut dispatcher = ParseEventDispatcher::new(1, ParseMode::ByStatement, engine);

            'next_state: {
                assert_eq!(Err(ParseEventError::NoGotoCandidate { state: 1, lhs: "EOF".into() }), dispatcher.next(Some(syntax_kind::r#SEMI)));
                assert_eq!(vec![1], dispatcher.state_values());
                break 'next_state;
            }

            Ok(())
        }

        static DUMMY_LA_TRANSITION: Transition = Transition::Reduce { pop_count: 0, lhs: 1 };
        fn next_lookahead_translation(_kind: u32, _state: usize) -> Option<&'static Transition> {
            Some(&DUMMY_LA_TRANSITION)
        }
    }
}

mod dispatcher_support_tests {
    use std::collections::VecDeque;

    use engine_core::{scanner_engine::ScanEvent, SyntaxKind};
    use parser_core::{error_recovery::{RecoveryEvent, RecoveryEventDispatcher, RecoveryEventPayload, RecoveryPenalty}, event_dispatcher::{ParseEvent, ParseEventDispatcher}, ParseMode};
    use scanner_core::{iter::LookaheadIterator, Token};
    use sqlite_engine::syntax_kind;

    fn prepare_dispatcher_state(dispatcher: &mut ParseEventDispatcher, requests: &[(SyntaxKind, usize)]) -> Result<(), anyhow::Error> {
        for req in requests.iter().flat_map(|(ev, n)| std::iter::repeat(*ev).take(*n)) {
            dispatcher.next(Some(req))?;
        }

        Ok(())
    }

    #[test]
    fn test_borrow_stack() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?.parsing_rules;
        let mut dispatcher = ParseEventDispatcher::new(0, ParseMode::ByStatement, engine);

        prepare_dispatcher_state(&mut dispatcher, &[
            (syntax_kind::SELECT, 1),
            (syntax_kind::STAR, 4),
            (syntax_kind::FROM, 1),
        ])?;

        let state_stack = dispatcher.borrow_stack();

        let expected_state = vec![145, 71, 18, 0];
        assert_eq!(expected_state, state_stack.state_values());
        Ok(())
    }

    #[test]
    fn test_post_delete_recovery_event() -> Result<(), anyhow::Error>{
        let engine = sqlite_engine::create()?.parsing_rules;
        let mut dispatcher = ParseEventDispatcher::new(0, ParseMode::ByStatement, engine);

        prepare_dispatcher_state(&mut dispatcher, &[
            (syntax_kind::SELECT, 1),
            (syntax_kind::INTEGER, 4),
        ])?;

        let mut lookaheads = vec![
            ScanEvent{ kind:syntax_kind::BLOB, offset: 10, len: 6, value: Some("x'quv'".into()) },
            ScanEvent{ kind:syntax_kind::INTEGER, offset: 17, len: 3, value: Some("101".into()) },
            ScanEvent{ kind:syntax_kind::FROM, offset: 21, len: 4, value: Some("FROM".into()) },
            ScanEvent{ kind:syntax_kind::ID, offset: 26, len: 1, value: Some("x".into()) }
        ].into_iter().peekable();

        let events = vec![
            RecoveryEvent::PatchDelete { kind: syntax_kind::BLOB, state: 122 },
            RecoveryEvent::PatchDelete { kind: syntax_kind::INTEGER, state: 122 },
            RecoveryEvent::Stitch(RecoveryEventPayload::Reduce{
                kind: syntax_kind::term, state: 122, next_state: 128, pop_count: 1,
            }),
        ];

        dispatcher.post_recovery_event(&events);

        'next_state: {
            let lookahead = lookaheads.peek().map(|x| x.kind);
            lookaheads.next();
            let event = dispatcher.next(lookahead)?;
            let expect_event = ParseEvent::PatchDrop { kind: syntax_kind::BLOB, current_state: 122, next_state: 122, edit_state: 122 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }
        'next_state: {
            let lookahead = lookaheads.peek().map(|x| x.kind);
            lookaheads.next();
            let event = dispatcher.next(lookahead)?;
            let expect_event = ParseEvent::PatchDrop { kind: syntax_kind::INTEGER, current_state: 122, next_state: 122, edit_state: 122 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }
        'next_state: {
            let lookahead = lookaheads.peek().map(|x| x.kind);
            let event = dispatcher.next(lookahead)?;
            let expect_event = ParseEvent::Reduce { kind: syntax_kind::term, current_state: 122, next_state: 128, edit_state: 238, pop_count: 1 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }
        'next_state: {
            let lookahead = lookaheads.peek().map(|x| x.kind);
            let event = dispatcher.next(lookahead)?;
            let expect_event = ParseEvent::Reduce { kind: syntax_kind::expr, current_state: 128, next_state: 361, edit_state: 238, pop_count: 1 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }
        'next_state: {
            let lookahead = lookaheads.peek().map(|x| x.kind);
            let event = dispatcher.next(lookahead)?;
            let expect_event = ParseEvent::Reduce { kind:syntax_kind::scanpt, current_state:361, next_state:467, edit_state:361, pop_count:0 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }
        'next_state: {
            let lookahead = lookaheads.peek().map(|x| x.kind);
            let event = dispatcher.next(lookahead)?;
            let expect_event = ParseEvent::Reduce { kind:syntax_kind::r#as, current_state:467, next_state:579, edit_state:467, pop_count:0 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }
        'next_state: {
            let lookahead = lookaheads.peek().map(|x| x.kind);
            let event = dispatcher.next(lookahead)?;
            let expect_event = ParseEvent::Reduce { kind:syntax_kind::selcollist, current_state:579, next_state:145, edit_state:18, pop_count:5 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }
        'next_state: {
            let lookahead = lookaheads.peek().map(|x| x.kind);
            lookaheads.next();
            let event = dispatcher.next(lookahead)?;
            let expect_event = ParseEvent::Shift { kind: syntax_kind::FROM, current_state:145, next_state:240, edit_state:145 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }

        Ok(())
    }

    #[test]
    fn test_post_shift_recovery_event() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?.parsing_rules;
        let mut dispatcher = ParseEventDispatcher::new(0, ParseMode::ByStatement, engine);

        prepare_dispatcher_state(&mut dispatcher, &[
            (syntax_kind::SELECT, 1),
            (syntax_kind::INTEGER, 4),
        ])?;

        let mut lookaheads = vec![
            ScanEvent{ kind:syntax_kind::INTEGER, offset: 10, len: 3, value: Some("101".into()) },
            ScanEvent{ kind:syntax_kind::AS, offset: 15, len: 2, value: Some("AS".into()) },
            ScanEvent{ kind:syntax_kind::ID, offset: 18, len: 1, value: Some("x".into()) }
        ].into_iter().peekable();

        let events = vec![
            RecoveryEvent::PatchShift(RecoveryEventPayload::Reduce {kind:syntax_kind::term, state:122, next_state:128, pop_count: 1 }),
            RecoveryEvent::PatchShift(RecoveryEventPayload::Reduce {kind:syntax_kind::expr, state:128, next_state:361, pop_count: 1 }),
            RecoveryEvent::PatchShift(RecoveryEventPayload::Shift {kind:syntax_kind::STAR, state:361, next_state:214 }),
            RecoveryEvent::Stitch(RecoveryEventPayload::Shift { kind:syntax_kind::INTEGER, state:214, next_state:122 }),
            RecoveryEvent::Stitch(RecoveryEventPayload::Reduce { kind:syntax_kind::term, state:122, next_state:128, pop_count: 1 })
        ];

        dispatcher.post_recovery_event(&events);

        'next_state: {
            let lookahead = lookaheads.peek().map(|x| x.kind);
            let event = dispatcher.next(lookahead)?;
            let expect_event = ParseEvent::PatchReduce { kind: syntax_kind::term, current_state: 122, next_state: 128, edit_state: 238, pop_count: 1 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }
        'next_state: {
            let lookahead = lookaheads.peek().map(|x| x.kind);
            let event = dispatcher.next(lookahead)?;
            let expect_event = ParseEvent::PatchReduce { kind: syntax_kind::expr, current_state: 128, next_state: 361, edit_state: 238, pop_count: 1 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }
        'next_state: {
            let lookahead = lookaheads.peek().map(|x| x.kind);
            let event = dispatcher.next(lookahead)?;
            let expect_event = ParseEvent::PatchShift { kind: syntax_kind::STAR, current_state: 361, next_state: 214, edit_state: 361 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }
        'next_state: {
            let lookahead = lookaheads.peek().map(|x| x.kind);
            lookaheads.next();
            let event = dispatcher.next(lookahead)?;
            let expect_event = ParseEvent::Shift { kind: syntax_kind::INTEGER, current_state: 214, next_state: 122, edit_state: 214 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }
        'next_state: {
            let lookahead = lookaheads.peek().map(|x| x.kind);
            let event = dispatcher.next(lookahead)?;
            let expect_event = ParseEvent::Reduce { kind: syntax_kind::term, current_state: 122, next_state: 128, edit_state: 214, pop_count: 1 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }
        'next_state: {
            let lookahead = lookaheads.peek().map(|x| x.kind);
            let event = dispatcher.next(lookahead)?;
            let expect_event = ParseEvent::Reduce { kind: syntax_kind::expr, current_state: 128, next_state: 326, edit_state: 214, pop_count: 1 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }
        'next_state: {
            let lookahead = lookaheads.peek().map(|x| x.kind);
            let event = dispatcher.next(lookahead)?;
            let expect_event = ParseEvent::Reduce { kind: syntax_kind::expr, current_state: 326, next_state: 361, edit_state: 238, pop_count: 3 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }
        'next_state: {
            let lookahead = lookaheads.peek().map(|x| x.kind);
            let event = dispatcher.next(lookahead)?;
            let expect_event = ParseEvent::Reduce { kind: syntax_kind::scanpt, current_state: 361, next_state: 467, edit_state: 361, pop_count: 0 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }
        'next_state: {
            let lookahead = lookaheads.peek().map(|x| x.kind);
            lookaheads.next();
            let event = dispatcher.next(lookahead)?;
            let expect_event = ParseEvent::Shift { kind: syntax_kind::AS, current_state: 467, next_state: 576, edit_state: 467 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }

        Ok(())
    }

    #[test]
    fn test_post_invalid_recovery_event() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?.parsing_rules;
        let mut dispatcher = ParseEventDispatcher::new(0, ParseMode::ByStatement, engine);
        let penalty = RecoveryPenalty{ delete_slot: 0, shift_limit: 0, shift_decay: 0, next_shift_decay: 0, max_shift_packet_size: 0 };
        let recovery_handler = RecoveryEventDispatcher::new(penalty, engine);

        prepare_dispatcher_state(&mut dispatcher, &[
            (syntax_kind::SELECT, 1),
            (syntax_kind::INTEGER, 3),
        ])?;

        let lookaheads = VecDeque::from([
            Token{   
                main: ScanEvent{ kind:syntax_kind::INTEGER, offset: 10, len: 3, value: Some("101".into()) }, 
                leading_trivia: None, trailing_trivia: None,
            },
            Token{   
                main: ScanEvent{ kind:syntax_kind::AS, offset: 15, len: 2, value: Some("AS".into()) }, 
                leading_trivia: None, trailing_trivia: None,
            },
            Token{   
                main: ScanEvent{ kind:syntax_kind::ID, offset: 18, len: 1, value: Some("x".into()) }, 
                leading_trivia: None, trailing_trivia: None,
            },
            Token{   
                main: ScanEvent{ kind:syntax_kind::SEMI, offset: 19, len: 1, value: Some(";".into()) }, 
                leading_trivia: None, trailing_trivia: None,
            },
        ]);

        let recover_events = recovery_handler.handle_as_invalid(LookaheadIterator::new(&lookaheads, 0, lookaheads.len()));
        dispatcher.post_recovery_event(&recover_events);

        'next_state: {
            let event = dispatcher.next(None)?;
            let expect_event = ParseEvent::Invalid { kind: syntax_kind::INTEGER, current_state: 238, edit_state: 0 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }
        'next_state: {
            let event = dispatcher.next(None)?;
            let expect_event = ParseEvent::Invalid { kind: syntax_kind::AS, current_state: 238, edit_state: 0 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }
        'next_state: {
            let event = dispatcher.next( None)?;
            let expect_event = ParseEvent::Invalid { kind: syntax_kind::ID, current_state: 238, edit_state: 0 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }
        'next_state: {
            let event = dispatcher.next(None)?;
            let expect_event = ParseEvent::Invalid { kind: syntax_kind::SEMI, current_state: 238, edit_state: 0 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }
        'next_state: {
            let event = dispatcher.next(None)?;
            let expect_event = ParseEvent::Emit { kind: syntax_kind::ecmd, edit_state: 0 };
            assert_eq!(expect_event, event);
            break 'next_state;
        }

        Ok(())
    }
}