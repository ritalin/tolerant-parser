
#[cfg(test)]
mod delete_recovery_tests {
    use engine_core::scanner_engine::ScanEvent;
    use parser_core::{error_recovery::{delete_recovery::DeleteErrorRecovery, stitch_handler::StitchRecoveryHandler, RecoveryEvent, RecoveryEventPayload, RecoveryPenalty, RecoveryReport}, Recovery};
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

        let mut handler = DeleteErrorRecovery::new(0, state_histories, penalty, engine.parsing_rules);
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
        let failed_state = 23;
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

        let mut handler = DeleteErrorRecovery::new(failed_state, state_histories, penalty, engine.parsing_rules);
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
        assert_eq!(Recovery::Delete, report.method());
        assert_eq!(2, report.score());
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
        let failed_state = 23;
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

        let mut handler = DeleteErrorRecovery::new(failed_state, state_histories, penalty, engine.parsing_rules);
        let report = handler.handle(lookaheads.iter());
        assert_eq!(None, report);
        assert_eq!(0, handler.left_slot());
        Ok(())
    }

    #[test]
    fn test_stitch_delete_recovery_report() -> Result<(), anyhow::Error> {
        let engine = sqlite_engine::create()?;
        let state_histories = &[0, 238, 122];
        let failed_state = 122;

        let lookaheads = [
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::COMMA, offset: 8, len: 2, value: Some(",".into()) },
                trailing_trivia: None,
            },
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::STRING, offset: 12, len: 3, value: Some("'xyz'".into()) },
                trailing_trivia: None,
            },
            Token{
                leading_trivia: None,
                main: ScanEvent{ kind: syntax_kind::SEMI, offset: 15, len: 1, value: Some(";".into()) },
                trailing_trivia: None,
            },
        ];

        let mut report = RecoveryReport::new(failed_state, state_histories, Recovery::Delete);
        report.push_event(syntax_kind::INTEGER.id, RecoveryEvent::PatchDelete { 
            kind: syntax_kind::INTEGER, state: 122,  
        });
        report.push_event(syntax_kind::STRING.id, RecoveryEvent::PatchDelete { 
            kind: syntax_kind::STRING, state: 122, 
        });
        report.reset_score(2);

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

        assert_eq!(Recovery::Delete, stitch_report.method());
        assert_eq!(3, stitch_report.score());
        assert_eq!(expect_events, stitch_report.events());
        Ok(())
    }
}

#[cfg(test)]
mod shift_recovery_tests {
    use engine_core::scanner_engine::ScanEvent;
    use parser_core::{error_recovery::{shift_recovery::ShiftErrorRecovery, RecoveryEvent, RecoveryEventPayload, RecoveryPenalty}, Recovery};
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
        let failed_state = 212;
        let lookahead = Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::r#AS, offset: 16, len: 1, value: Some("AS".into()) },
            trailing_trivia: None,
        };

        let mut handler = ShiftErrorRecovery::new(failed_state, state_histories, penalty, engine.parsing_rules);
        let Some(report) = handler.handle(&lookahead) else {
            panic!("Actual value must be returned");
        };

        let expect_events: Vec<RecoveryEvent> = vec![
            RecoveryEvent::PatchShift(
                RecoveryEventPayload::Shift { kind: syntax_kind::ID, state: 212, next_state: 110 }
            ),
        ];

        assert_eq!(Recovery::Shift, report.method());
        assert_eq!(1, report.score());
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
        let failed_state = 212;
        let lookahead = Token{
            leading_trivia: None,
            main: ScanEvent{ kind: syntax_kind::r#AS, offset: 16, len: 1, value: Some("AS".into()) },
            trailing_trivia: None,
        };

        let mut handler = ShiftErrorRecovery::new(failed_state, state_histories, penalty, engine.parsing_rules);
        let report = handler.handle(&lookahead);
        assert_eq!(None, report);
        Ok(())
    }
}