use std::collections::VecDeque;

use engine_core::scanner_engine::{self, AcceptableRegexSet, ScanEvent};
use engine_core::SyntaxKind;
use crate::Token;
use crate::event_dispatch::ScanEventDispatcher;

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

    pub fn prefetch(&mut self, terminate_synbol: SyntaxKind) -> LookaheadIterator {
        // Find prefetch queue
        if let Some(p) = self.lookaheads.iter().position(|tk| tk.main.kind == terminate_synbol) {
            return LookaheadIterator::new(&self.lookaheads, p+1);
        }

        while let Some(next_lookahead) = handle_scan_event(&mut self.dispatcher) {
            match next_lookahead {
                lookahead if lookahead.main.kind.id == terminate_synbol.id => {
                    self.lookaheads.push_back(lookahead);
                    break;
                }
                lookahead => {
                    self.lookaheads.push_back(lookahead);
                }
            }
        }
        
        LookaheadIterator::new(&self.lookaheads, self.lookaheads.len())
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

#[derive(PartialEq, Clone, Debug)]
pub struct LookaheadIterator<'a> {
    inner: &'a VecDeque<Token>,
    index: usize,
    size: usize,
}

impl<'a> LookaheadIterator<'a> {
    pub fn new(lookaheads: &'a VecDeque<Token>, size: usize) -> Self {
        Self {
            inner: lookaheads,
            index: 0,
            size,
        }
    }

    pub fn peek(&self) -> Option<&'a Token> {
        self.inner.get(self.index)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'a> Iterator for LookaheadIterator<'a> {
    type Item = &'a Token;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.size {
            return None;
        }

        let token = self.inner.get(self.index);
        self.index += 1;

        token
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
    let main = match dispatcher.next(&AcceptableRegexSet::Main) {
        Some(event) => event,
        None if dispatcher.has_more() => dispatcher.invalid(),
        None => {
            return None;
        }
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