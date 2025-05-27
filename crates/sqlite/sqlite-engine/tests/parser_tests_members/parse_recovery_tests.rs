
#[cfg(test)]
mod delete_recovery_tests {
    use engine_core::scanner_engine::ScanEvent;
    use parser_core::error_recovery::{delete_recovery::DeleteErrorRecovery, stitch_handler::StitchRecoveryHandler, RecoveryEvent, RecoveryEventPayload, RecoveryPenalty, RecoveryReport};
    use scanner_core::Token;
    use sqlite_engine::syntax_kind;

    #[test]
    fn test_patch_delete_without_lookahead() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let penalty = RecoveryPenalty{
            delete_slot: 3,
            shift_limit: 0, shift_decay: 0, next_shift_decay: 0, max_shift_packet_size: 0
        };
        let state_histories = &[0];
        let lookaheads = [];

        let mut handler = DeleteErrorRecovery::new(state_histories, penalty, engine.parsing_rules);
        let report = handler.handle(lookaheads.into_iter());

        assert_eq!(None, report);
        assert_eq!(3, handler.left_slot());
        Ok(())
    }

    #[test]
    fn test_patch_delete() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let penalty = RecoveryPenalty{
            delete_slot: 3,
            shift_limit: 0, shift_decay: 0, next_shift_decay: 0, max_shift_packet_size: 0
        };
        let state_histories = &[0, 23];
        let lookaheads = [
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 8, len: 2, value: Some("42".into()) },
                trailing_trivia: None,
            },
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::STRING, offset: 12, len: 3, value: Some("'abc'".into()) },
                trailing_trivia: None,
            },
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::SEMI, offset: 15, len: 1, value: Some(";".into()) },
                trailing_trivia: None,
            },
        ];

        let mut handler = DeleteErrorRecovery::new(state_histories, penalty, engine.parsing_rules);
        let Some(report) = handler.handle(lookaheads.iter()) else {
            panic!("Actual value must be returned");
        };

        let expect_events = vec![
            RecoveryEvent::PatchDelete { 
                kind: syntax_kind::INTEGER, state: 23, 
            },
            RecoveryEvent::PatchDelete { 
                kind: syntax_kind::STRING, state: 23, 
            },
        ];
        assert_eq!(2, report.patch_score());
        assert_eq!(expect_events, report.events());
        assert_eq!(1, handler.left_slot());
        Ok(())
    }

    #[test]
    fn test_patch_delete_for_penalty_violation() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let penalty = RecoveryPenalty{
            delete_slot: 1,
            shift_limit: 0, shift_decay: 0, next_shift_decay: 0, max_shift_packet_size: 0
        };
        let state_histories = &[0, 23];
        let lookaheads = [
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 8, len: 2, value: Some("42".into()) },
                trailing_trivia: None,
            },
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::STRING, offset: 12, len: 3, value: Some("'abc'".into()) },
                trailing_trivia: None,
            },
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::SEMI, offset: 15, len: 1, value: Some(";".into()) },
                trailing_trivia: None,
            },
        ];

        let mut handler = DeleteErrorRecovery::new(state_histories, penalty, engine.parsing_rules);
        let report = handler.handle(lookaheads.iter());
        assert_eq!(None, report);
        assert_eq!(0, handler.left_slot());
        Ok(())
    }

    #[test]
    fn test_stitch_delete_recovery_report() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let state_histories = &[0, 238, 122];

        let lookaheads = [
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::COMMA, offset: 8, len: 1, value: Some(",".into()) },
                trailing_trivia: None,
            },
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::STRING, offset: 9, len: 3, value: Some("'xyz'".into()) },
                trailing_trivia: None,
            },
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::SEMI, offset: 12, len: 1, value: Some(";".into()) },
                trailing_trivia: None,
            },
        ];

        let mut report = RecoveryReport::new(state_histories);
        report.push_event(syntax_kind::INTEGER.id, RecoveryEvent::PatchDelete { 
            kind: syntax_kind::INTEGER, state: 122,  
        });
        report.push_event(syntax_kind::STRING.id, RecoveryEvent::PatchDelete { 
            kind: syntax_kind::STRING, state: 122, 
        });

        let handler = StitchRecoveryHandler::new(engine.parsing_rules);
        let Some(stitch_report) = handler.try_recovery(report, lookaheads.iter()) else {
            panic!("Actual value must be returned");
        };

        let expect_events = vec![
            RecoveryEvent::PatchDelete{ 
                kind: syntax_kind::INTEGER, state: 122, 
            },
            RecoveryEvent::PatchDelete{ 
                kind: syntax_kind::STRING, state: 122, 
            },
            RecoveryEvent::Stitch(RecoveryEventPayload::Reduce{ 
                kind: syntax_kind::term, state: 122, next_state: 128, pop_count: 1
            })
        ];

        assert_eq!(1, stitch_report.stitch_score());
        assert_eq!(expect_events, stitch_report.events());
        Ok(())
    }
}

#[cfg(test)]
mod shift_recovery_tests {
    use engine_core::scanner_engine::ScanEvent;
    use parser_core::error_recovery::{shift_recovery::ShiftErrorRecovery, stitch_handler::StitchRecoveryHandler, RecoveryEvent, RecoveryEventPayload, RecoveryPenalty, RecoveryReport};
    use scanner_core::Token;
    use sqlite_engine::syntax_kind;

    #[test]
    fn test_patch_shift() -> Result<(), anyhow::Error> {
        let engine = engine_core::Engine {
            scanning_rules: sqlite_engine::builder::scan_rule_builder().build()?,
            parsing_rules: sqlite_engine::builder::parse_rule_builder()
                .candidate_symbols(|state| {
                    let mut symbols = sqlite_engine::builder::get_candidate_symbols(state);
                    symbols.sort_by(|lhs, rhs| lhs.cmp(rhs));
                    symbols
                })
                .build()?,
        };
        let penalty = RecoveryPenalty{
            delete_slot: 0,
            shift_limit: 4, shift_decay: 0, next_shift_decay: 1, max_shift_packet_size: 10,
        };
        let state_histories = &[128, 361, 212];
        let lookahead = Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::r#AS, offset: 16, len: 2, value: Some("AS".into()) },
            trailing_trivia: None,
        };

        let mut handler = ShiftErrorRecovery::new(state_histories, penalty, engine.parsing_rules);
        let Some(report) = handler.handle(&lookahead) else {
            panic!("Actual value must be returned");
        };

        let expect_events: Vec<RecoveryEvent> = vec![
            RecoveryEvent::PatchShift(
                RecoveryEventPayload::Shift { kind: syntax_kind::ID, state: 212, next_state: 110 }
            ),
        ];

        assert_eq!(1, report.patch_score());
        assert_eq!(expect_events, report.events());
        Ok(())
    }

    #[test]
    fn test_patch_shift_for_penalty_violation() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let penalty = RecoveryPenalty{
            delete_slot: 0,
            shift_limit: 0, shift_decay: 0, next_shift_decay: 1, max_shift_packet_size: 10,
        };

        let state_histories = &[128, 361, 212];
        let lookahead = Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::r#AS, offset: 16, len: 2, value: Some("AS".into()) },
            trailing_trivia: None,
        };

        let mut handler = ShiftErrorRecovery::new(state_histories, penalty, engine.parsing_rules);
        let report = handler.handle(&lookahead);
        assert_eq!(None, report);
        Ok(())
    }

    #[test]
    fn test_patch_shift_recovery_including_reduce() -> Result<(), anyhow::Error> {
        let engine = engine_core::Engine {
            scanning_rules: sqlite_engine::builder::scan_rule_builder().build()?,
            parsing_rules: sqlite_engine::builder::parse_rule_builder()
                .candidate_symbols(|state| {
                    match state {
                        122 => vec![&syntax_kind::STAR],
                        _ => sqlite_engine::builder::get_candidate_symbols(state)
                    }
                })
                .build()?,
        };
        let penalty = RecoveryPenalty{
            delete_slot: 0,
            shift_limit: 4, shift_decay: 0, next_shift_decay: 1, max_shift_packet_size: 10,
        };
        let state_histories = &[0, 108, 122];
        let lookahead = Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 13, len: 2, value: Some("20".into()) },
            trailing_trivia: None,
        };

        let mut handler = ShiftErrorRecovery::new(state_histories, penalty, engine.parsing_rules);
        let Some(report) = handler.handle(&lookahead) else {
            panic!("Actual value must be returned");
        };

        let expect_events: Vec<RecoveryEvent> = vec![
            RecoveryEvent::PatchShift(
                RecoveryEventPayload::Reduce { kind: syntax_kind::term, state: 122, next_state: 128, pop_count: 1 }
            ),
            RecoveryEvent::PatchShift(
                RecoveryEventPayload::Reduce { kind: syntax_kind::expr, state: 128, next_state: 178, pop_count: 1 }
            ),
            RecoveryEvent::PatchShift(
                RecoveryEventPayload::Shift { kind: syntax_kind::STAR, state: 178, next_state: 214 }
            ),
        ];

        assert_eq!(3, report.patch_score());
        assert_eq!(expect_events, report.events());
        Ok(())
    }

    #[test]
    fn test_stitch_shift_recovery_report() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let state_histories = &[0, 214];

        let lookaheads = [
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 11, len: 2, value: Some("30".into()) },
                trailing_trivia: None,
            },
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::AS, offset: 14, len: 2, value: Some("AS".into()) },
                trailing_trivia: None,
            },
        ];

        let mut report = RecoveryReport::new(state_histories);
        report.push_event(syntax_kind::term.id, RecoveryEvent::PatchShift(
            RecoveryEventPayload::Reduce { kind: syntax_kind::term, state: 122, next_state: 128, pop_count: 1 }
        ));
        report.push_event(syntax_kind::expr.id, RecoveryEvent::PatchShift(
            RecoveryEventPayload::Reduce { kind: syntax_kind::expr, state: 128, next_state: 178, pop_count: 1 }
        ));
        report.push_event(syntax_kind::STAR.id, RecoveryEvent::PatchShift(
            RecoveryEventPayload::Shift { kind: syntax_kind::STAR, state: 178, next_state: 214 }
        ));

        let handler = StitchRecoveryHandler::new(engine.parsing_rules);
        let Some(stitch_report) = handler.try_recovery(report, lookaheads.iter()) else {
            panic!("Actual value must be returned");
        };

        let expect_events = vec![
            RecoveryEvent::PatchShift(RecoveryEventPayload::Reduce { 
                kind: syntax_kind::term, state: 122, next_state: 128, pop_count: 1 
            }),
            RecoveryEvent::PatchShift(RecoveryEventPayload::Reduce { 
                kind: syntax_kind::expr, state: 128, next_state: 178, pop_count: 1 
            }),
            RecoveryEvent::PatchShift(RecoveryEventPayload::Shift { 
                kind: syntax_kind::STAR, state: 178, next_state: 214 
            }),
            RecoveryEvent::Stitch(RecoveryEventPayload::Shift{ 
                kind: syntax_kind::INTEGER, state: 214, next_state: 122
            }),
            RecoveryEvent::Stitch(RecoveryEventPayload::Reduce{
                kind: syntax_kind::term, state: 122, next_state: 128, pop_count: 1
            })
        ];

        assert_eq!(2, stitch_report.stitch_score());
        assert_eq!(expect_events, stitch_report.events());
        Ok(())
    }
}

#[cfg(test)]
mod recovery_tests {
    use std::collections::VecDeque;

    use engine_core::scanner_engine::ScanEvent;
    use parser_core::error_recovery::{RecoveryEvent, RecoveryEventDispatcher, RecoveryEventPayload, RecoveryPenalty};
    use scanner_core::{LookaheadIterator, Token};
    use sqlite_engine::syntax_kind;

    #[test]
    fn test_recovery_with_deleting_candidateg() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let penalty = RecoveryPenalty{
            delete_slot: 3,
            shift_limit: 1, shift_decay: 0, next_shift_decay: 1, max_shift_packet_size: 10,
        };
        let state_histories = &[0, 238, 122];
        
        let lookaheads = VecDeque::from([
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 10, len: 3, value: Some("101".into()) },
                trailing_trivia: None,
            },
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::BLOB, offset: 14, len: 3, value: Some("x'xyz'".into()) },
                trailing_trivia: None,
            },
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 18, len: 3, value: Some("101".into()) },
                trailing_trivia: None,
            },
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::FROM, offset: 22, len: 4, value: Some("FROM".into()) },
                trailing_trivia: None,
            },
        ]);

        let mut handler = RecoveryEventDispatcher::new(penalty, engine.parsing_rules);
        let Some(events) = handler.handle_from_history(state_histories, LookaheadIterator::new(&lookaheads, lookaheads.len())) else {
            panic!("Actual value must be returned");
        };

        let expected_events = vec![
            RecoveryEvent::PatchDelete { 
                kind: syntax_kind::INTEGER, state: 122,  
            },
            RecoveryEvent::PatchDelete { 
                kind: syntax_kind::BLOB, state: 122,  
            },
            RecoveryEvent::PatchDelete { 
                kind: syntax_kind::INTEGER, state: 122,  
            },
            RecoveryEvent::Stitch(RecoveryEventPayload::Reduce {
                kind: syntax_kind::term, state: 122, next_state: 128, pop_count: 1
            }),
        ];

        let after_penalty = handler.penalty();
        assert_eq!(0, after_penalty.delete_slot);
        assert_eq!(1, after_penalty.shift_limit);
        assert_eq!(0, after_penalty.shift_decay);
        assert_eq!(1, after_penalty.next_shift_decay);

        assert_eq!(expected_events, events);
        
        Ok(())
    }

    #[test]
    fn test_recovery_with_shifting_candidate() -> Result<(), anyhow::Error> {
        let engine = engine_core::Engine {
            scanning_rules: sqlite_engine::builder::scan_rule_builder().build()?,
            parsing_rules: sqlite_engine::builder::parse_rule_builder()
                .candidate_symbols(|state| {
                    let mut candidates = sqlite_engine::builder::get_candidate_symbols(state);
                    if let Some(i) = candidates.iter().position(|x| x.id == syntax_kind::EQ.id) {
                        candidates.swap(0, i);
                    }
                    candidates
                })
                .build()?,
        };
        let penalty = RecoveryPenalty{
            delete_slot: 3,
            shift_limit: 10, shift_decay: 0, next_shift_decay: 1, max_shift_packet_size: 10,
        };
        let state_histories = &[0, 238, 122];

        let lookaheads = VecDeque::from([
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 11, len: 2, value: Some("30".into()) },
                trailing_trivia: None,
            },
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::AS, offset: 14, len: 2, value: Some("AS".into()) },
                trailing_trivia: None,
            },
        ]);

        let mut handler = RecoveryEventDispatcher::new(penalty, engine.parsing_rules);
        let Some(events) = handler.handle_from_history(state_histories, LookaheadIterator::new(&lookaheads, lookaheads.len())) else {
            panic!("Actual value must be returned");
        };

        let expected_events = vec![
            RecoveryEvent::PatchShift(RecoveryEventPayload::Reduce {
                kind: syntax_kind::term, state: 122, next_state: 128, pop_count: 1
            }),
            RecoveryEvent::PatchShift(RecoveryEventPayload::Reduce {
                kind: syntax_kind::expr, state: 128, next_state: 361, pop_count: 1
            }),
            RecoveryEvent::PatchShift(RecoveryEventPayload::Shift { 
                kind: syntax_kind::EQ, state: 361, next_state: 203 
            }),
            RecoveryEvent::Stitch(RecoveryEventPayload::Shift { 
                kind: syntax_kind::INTEGER, state: 203, next_state: 122 
            }),
            RecoveryEvent::Stitch(RecoveryEventPayload::Reduce {
                kind: syntax_kind::term, state: 122, next_state: 128, pop_count: 1
            }),
        ];
        
        assert_eq!(expected_events, events);

        Ok(())
    }

    #[test]
    fn test_recovery_without_candidate() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let penalty = RecoveryPenalty{
            delete_slot: 2,
            shift_limit: 1, shift_decay: 0, next_shift_decay: 1, max_shift_packet_size: 10,
        };
        let state_histories = &[0, 238, 122];

        let lookaheads = &VecDeque::from([
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 10, len: 3, value: Some("101".into()) },
                trailing_trivia: None,
            },
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::BLOB, offset: 14, len: 3, value: Some("x'xyz'".into()) },
                trailing_trivia: None,
            },
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::INTEGER, offset: 18, len: 3, value: Some("101".into()) },
                trailing_trivia: None,
            },
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::FROM, offset: 22, len: 4, value: Some("FROM".into()) },
                trailing_trivia: None,
            },
        ]);

        let mut handler = RecoveryEventDispatcher::new(penalty, engine.parsing_rules);
        let events = handler.handle_from_history(state_histories, LookaheadIterator::new(&lookaheads, lookaheads.len()));

        assert_eq!(None, events);

        let after_penalty = handler.penalty();
        assert_eq!(2, after_penalty.delete_slot);
        assert_eq!(1, after_penalty.shift_limit);
        assert_eq!(0, after_penalty.shift_decay);
        assert_eq!(1, after_penalty.next_shift_decay);

        Ok(())
    }

    #[test]
    fn test_handle_event_as_invalid() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let penalty = RecoveryPenalty{
            delete_slot: 2,
            shift_limit: 1, shift_decay: 0, next_shift_decay: 1, max_shift_packet_size: 10,
        };

        let lookaheads = VecDeque::from([
            Token{
                leading_trivia: Some(vec![
                    ScanEvent{ kind: syntax_kind::SPACE, offset: 0, len: 2, value: Some("  ".into()) }
                ]),
                main: ScanEvent { kind: syntax_kind::SELECT, offset: 2, len: 6, value: Some("SELECT".into()) },
                trailing_trivia: Some(vec![
                    ScanEvent{ kind: syntax_kind::SPACE, offset: 8, len: 1, value: Some(" ".into()) }
                ])
            },
            Token{
                leading_trivia: None,
                main: ScanEvent { kind: syntax_kind::INTEGER, offset: 9, len: 1, value: Some("9".into()) },
                trailing_trivia: Some(vec![
                    ScanEvent{ kind: syntax_kind::SPACE, offset: 10, len: 1, value: Some(" ".into()) }
                ])
            },
            Token{
                leading_trivia: None,
                main: ScanEvent { kind: syntax_kind::FROM, offset: 11, len: 4, value: Some("FROM".into()) },
                trailing_trivia: None
            },
        ]);

        let handler = RecoveryEventDispatcher::new(penalty, engine.parsing_rules);
        let events = handler.handle_as_invalid(LookaheadIterator::new(&lookaheads, lookaheads.len()), true);

        let expect_events = vec![
            RecoveryEvent::Invalid { kind: syntax_kind::SELECT, need_emit: false },
            RecoveryEvent::Invalid { kind: syntax_kind::INTEGER, need_emit: false },
            RecoveryEvent::Invalid { kind: syntax_kind::FROM, need_emit: true }
        ];

        assert_eq!(expect_events, events);
        Ok(())
    }
}