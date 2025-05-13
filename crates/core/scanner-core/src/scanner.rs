use std::collections::VecDeque;

use engine_core::scanner_engine::{self, AcceptableRegexSet, ScanEvent};
use crate::Token;
use crate::dispatch::ScanEventDispatcher;

pub struct Scanner {
    lookaheads: VecDeque<Token>,
    dispatcher: ScanEventDispatcher,
}

impl Scanner {
    /// Create new scanner instance
    pub fn create(source: &str, index: u32, engine: scanner_engine::ScanningRuleSet) -> Result<Self, crate::ScannerError> {
        let mut dispatcher = ScanEventDispatcher::new(source, index, engine);
        let lookahead = handle_scan_event(&mut dispatcher).ok_or(crate::ScannerError::CreateFailed)?;

        Ok(Self { dispatcher, lookaheads: VecDeque::from_iter([lookahead].into_iter()) })
    }

    /// Peek current lookahead
    pub fn lookahead(&self) -> Option<&Token> {
        self.lookaheads.front()
    }

    /// Return current lookahead and proceed lookahead
    pub fn shift(&mut self) -> Option<Token> {
        let lookahead = self.lookaheads.pop_front();
        if self.lookaheads.is_empty() {
            if let Some(next_lookahead) = handle_scan_event(&mut self.dispatcher) {
                self.lookaheads.push_back(next_lookahead);
            }
        }

        lookahead
    }

    pub fn save_scope(&self) -> ScannerScope {
        ScannerScope::new()
    }

    pub fn restore_scope(&mut self, scope: ScannerScope) {
        // restore cached lookaheads
        for token in scope.lookaheads.into_iter().rev() {
            self.lookaheads.push_front(token);
        }
    }
}

pub struct ScannerScope {
    lookaheads: Vec<Token>,
}

impl ScannerScope {
    pub fn new() -> Self {
        Self { lookaheads: Default::default()}
    }

    pub fn cache_lookahead(&mut self, lookahead: Option<Token>) -> Option<Token> {
        if let Some(token) = lookahead.as_ref() {
            self.lookaheads.push(token.clone());
        }

        lookahead
    }
}

fn handle_scan_event(dispatcher: &mut ScanEventDispatcher) -> Option<Token> {
    // scan leading trivia
    let leading_trivia = handle_scan_trivia_event(dispatcher, AcceptableRegexSet::Leading);
    // scan main token
    let Some(main) = dispatcher.next(&AcceptableRegexSet::Main) else {
        return None;
    };
    // scan trailing trivia
    let trailing_trivia = handle_scan_trivia_event(dispatcher, AcceptableRegexSet::Trailing);

    Some(Token { leading_trivia, main, trailing_trivia })
}

fn handle_scan_trivia_event(dispatcher: &mut ScanEventDispatcher, regex_set: AcceptableRegexSet) -> Option<Vec<ScanEvent>> {
    let mut trivias = vec![];

    while let Some(event) = dispatcher.next_regex(&regex_set) {
        trivias.push(event);
    }
    
    (trivias.len() > 0).then(|| trivias)
}