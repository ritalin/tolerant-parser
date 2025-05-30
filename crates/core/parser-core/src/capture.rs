use std::collections::VecDeque;

use engine_core::{Engine, SyntaxKind};
use scanner_core::{Scanner, Token};

use crate::{error_recovery::{RecoveryEventDispatcher, RecoveryPenalty}, event_dispatcher::{ParseEvent, ParseEventDispatcher, ParseEventError}, parser::{ParseError, ParseMode}};

#[derive(Clone)]
pub struct EventCaptureConfig {
    pub mode: ParseMode,
    pub no_scan: bool,
    pub no_parse: bool,
}

#[derive(Clone)]
pub enum  CaptureEvent {
    Scan(Token),
    Parse(ParseEvent),
}

pub struct ParseEventCapture {
    scanner: Scanner,
    dispatcher: ParseEventDispatcher,
    recovery_handler: RecoveryEventDispatcher,
    config: EventCaptureConfig,
    event_queue: VecDeque<Option<CaptureEvent>>,
    stmt_term_kind: SyntaxKind,
    accepted: bool,
}

impl ParseEventCapture {
    pub fn create(source: &str, config: EventCaptureConfig, engine: Engine) -> Result<Self, ParseError> {
        let scanner = Scanner::create(source.into(), 0, engine.scanning_rules)?;
        let event_queue = match scanner.lookahead() {
            Some(lookahead) if ! config.no_scan => VecDeque::from([Some(CaptureEvent::Scan(lookahead.clone()))]),
            _ => VecDeque::new(),
        };
        let penalty = RecoveryPenalty{ delete_slot: 3, shift_limit: 10, shift_decay: 0, next_shift_decay: 1, max_shift_packet_size: 10 };
        let stmt_term_kind = engine.parsing_rules.statement_emit_config().to_symbol;

        let this = Self { 
            scanner, 
            dispatcher: ParseEventDispatcher::new(0, config.mode.clone(), engine.parsing_rules),
            recovery_handler: RecoveryEventDispatcher::new(penalty, engine.parsing_rules),
            config: config,
            event_queue,
            stmt_term_kind,
            accepted: false,
        };

        Ok(this)
    }

    pub fn next(&mut self) -> Result<Option<CaptureEvent>, ParseError> {
        if self.accepted {
            return Ok(None);
        }

        while self.event_queue.is_empty() {
            let lookahead = match self.scanner.lookahead().cloned() {
                Some(lookahead) => Some(lookahead.main.kind),
                None if self.dispatcher.has_next() => None,
                None if self.config.mode == ParseMode::Full => None,
                None => break,
            };
            let event = match self.dispatcher.next(lookahead) {
                Ok(event) => event,
                Err(ParseEventError::RequestRecovery) => {
                    self.try_recover()?;
                    self.dispatcher.next(lookahead)?
                }
                Err(err) => return Err(ParseError::ByEvent(err)),
            };

            match event {
                ParseEvent::Shift { .. } => {
                    self.scanner.shift();

                    if !self.config.no_parse {
                        self.event_queue.push_back(Some(CaptureEvent::Parse(event)));
                    }
                    if !self.config.no_scan {
                        if let Some(lookahead) = self.scanner.lookahead() {
                            self.event_queue.push_back(Some(CaptureEvent::Scan(lookahead.clone())));
                        }
                    }
                }
                ParseEvent::Emit { .. } => {
                    if !self.config.no_parse {
                        self.event_queue.push_back(Some(CaptureEvent::Parse(event)));
                    }
                    if self.config.mode == ParseMode::ByStatement {
                        self.dispatcher.flush_state();
                    }
                }
                ParseEvent::Accept { .. } => {
                    if !self.config.no_parse {
                        self.event_queue.push_back(Some(CaptureEvent::Parse(event)));
                    }
                    self.accepted = true;
                    break;
                }
                ParseEvent::PatchDrop { .. } | ParseEvent::Invalid { .. } => {
                    self.scanner.shift();

                    if !self.config.no_parse {
                        self.event_queue.push_back(Some(CaptureEvent::Parse(event)));
                    }
                    if !self.config.no_scan {
                        if let Some(lookahead) = self.scanner.lookahead() {
                            self.event_queue.push_back(Some(CaptureEvent::Scan(lookahead.clone())));
                        }
                    }
                }
                event => {
                    if !self.config.no_parse {
                        self.event_queue.push_back(Some(CaptureEvent::Parse(event)));
                    }
                }
            }
        }

        Ok(self.event_queue.pop_front().flatten())
    }

    fn try_recover(&mut self) -> Result<(), ParseError> {
        let state_stack = self.dispatcher.borrow_stack();
        let prefetch = self.scanner.prefetch(self.stmt_term_kind);

        match self.recovery_handler.handle(state_stack, prefetch.clone()) {
            Some(events) => {
                self.dispatcher.post_recovery_event(&events);
            }
            None => {
                self.dispatcher.post_recovery_event(&self.recovery_handler.handle_as_invalid(prefetch));
            }
        }

        Ok(())
    }
}