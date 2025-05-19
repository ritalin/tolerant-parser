use std::collections::VecDeque;

use engine_core::Engine;
use scanner_core::{Scanner, Token};

use crate::{event_dispatcher::{ParseEvent, ParseEventDispatcher}, parser::ParseError};



#[derive(Clone)]
pub struct EventCaptureConfig {
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
    config: EventCaptureConfig,
    event_queue: VecDeque<Option<CaptureEvent>>,
    accepted: bool,
}

impl ParseEventCapture {
    pub fn create(source: &str, config: EventCaptureConfig, engine: Engine) -> Result<Self, ParseError> {
        let scanner = Scanner::create(source.into(), 0, engine.scanning_rules)?;
        let event_queue = match scanner.lookahead() {
            Some(lookahead) if ! config.no_scan => VecDeque::from([Some(CaptureEvent::Scan(lookahead.clone()))]),
            _ => VecDeque::new(),
        };

        let this = Self { 
            scanner, 
            dispatcher: ParseEventDispatcher::new(0, engine.parsing_rules),
            config: config,
            event_queue,
            accepted: false,
        };

        Ok(this)
    }

    pub fn next(&mut self) -> Result<Option<CaptureEvent>, ParseError> {
        if self.accepted {
            return Ok(None);
        }

        while self.event_queue.is_empty() {
            let event = match self.scanner.lookahead().cloned() {
                Some(lookahead) => self.dispatcher.next(Some(lookahead.main.kind)),
                None if self.dispatcher.has_next() => self.dispatcher.next(None),
                None => break,
            }?;

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
                    self.dispatcher.flush_state();
                }
                ParseEvent::Accept { .. } => {
                    if !self.config.no_parse {
                        self.event_queue.push_back(Some(CaptureEvent::Parse(event)));
                    }
                    self.accepted = true;
                    break;
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
}