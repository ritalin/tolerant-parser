use std::collections::VecDeque;

use crate::core::engine_core::scanner_engine::{self, AcceptableRegexSet, CaseSensitivity, ScanEvent};
use crate::core::engine_core::SyntaxKind;
use crate::core::scanner_core::{self, ScannerError, Token};
use crate::core::scanner_core::event_dispatch::ScanEventDispatcher;

pub struct Scanner {
    lookaheads: VecDeque<Token>,
    dispatcher: ScanEventDispatcher,
}

impl Scanner {
    /// Create new scanner instance
    pub fn create(source: &str, index: usize, engine: scanner_engine::ScanningRuleSet, case_sensitive: CaseSensitivity) -> Result<Self, ScannerError> {
        let mut dispatcher = ScanEventDispatcher::new(source, index, engine, case_sensitive, 0);
        let lookahead = handle_scan_event(&mut dispatcher).ok_or(ScannerError::CreateFailed)?;

        Ok(Self { dispatcher, lookaheads: VecDeque::from_iter([lookahead].into_iter()) })
    }

    pub fn create_without_scan(source: &str, index: usize, engine: scanner_engine::ScanningRuleSet, option: ScannerConfig) -> Result<Self, ScannerError> {
        let dispatcher = ScanEventDispatcher::new(source, index, engine, option.case_sensitive, option.offset_with);
        Ok(Self { dispatcher, lookaheads: VecDeque::new() })
        
    }

    pub fn statement_scanners(&self, emit_symbol: SyntaxKind, full_emit_symbol: SyntaxKind) -> scanner_core::iter::StatementScannerIterator {
        scanner_core::iter::StatementScannerIterator::new(
            self.lookaheads.clone(),
            self.dispatcher.clone(),
            emit_symbol,
            full_emit_symbol
        )
    }
}

pub struct ScannerConfig {
    pub case_sensitive: CaseSensitivity,
    pub offset_with: usize,
}

pub trait ScannerAccess {
    fn lookahead(&self) -> Option<&Token>;
    fn shift(&mut self) -> Option<Token>;
    fn prefetch_iter(&mut self, terminate_synbol: SyntaxKind) -> scanner_core::iter::LookaheadIterator;
}

impl ScannerAccess for Scanner {
    /// Peek current lookahead
    fn lookahead(&self) -> Option<&Token> {
        self.lookaheads.front()
    }

    /// Return current lookahead and proceed lookahead
    fn shift(&mut self) -> Option<Token> {
        let lookahead = self.lookaheads.pop_front();
        if self.lookaheads.is_empty() {
            if let Some(next_lookahead) = handle_scan_event(&mut self.dispatcher) {
                self.lookaheads.push_back(next_lookahead);
            }
        }

        lookahead
    }
    
    fn prefetch_iter(&mut self, terminate_synbol: SyntaxKind) -> scanner_core::iter::LookaheadIterator {
        // Find prefetch queue
        let len = prefetch_internal(terminate_synbol, &mut self.dispatcher, &mut self.lookaheads);
        scanner_core::iter::LookaheadIterator::new(&self.lookaheads, 0, len)
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

pub(crate) fn prefetch_internal(terminate_synbol: SyntaxKind, dispatcher: &mut ScanEventDispatcher, lookaheads: &mut VecDeque<Token>) -> usize {
    if let Some(p) = lookaheads.iter().position(|tk| tk.main.kind == terminate_synbol) {
        return p+1;
    }

    while let Some(next_lookahead) = handle_scan_event(dispatcher) {
        match next_lookahead {
            lookahead if lookahead.main.kind.id == terminate_synbol.id => {
                lookaheads.push_back(lookahead);
                break;
            }
            lookahead => {
                lookaheads.push_back(lookahead);
            }
        }
    }

    lookaheads.len()
}

pub struct StatementScannerView<'a> {
    lookaheads: &'a VecDeque<Token>,
    index: usize,
    end: usize,
}

impl<'a> StatementScannerView<'a> {
    pub fn new(lookaheads: &'a VecDeque<Token>, index: usize, size: usize) -> Self {
        Self {
            lookaheads,
            index,
            end: index + size,
        }
    }
}

impl<'a> ScannerAccess for StatementScannerView<'a> {
    fn lookahead(&self) -> Option<&Token> {
        if self.index >= self.end {
            return None;
        }

        self.lookaheads.get(self.index)
    }

    fn shift(&mut self) -> Option<Token> {
        if self.index >= self.end {
            return None;
        }

        let token = self.lookaheads.get(self.index).cloned();
        self.index += 1;
        token
    }

    fn prefetch_iter(&mut self, _terminate_synbol: SyntaxKind) -> scanner_core::iter::LookaheadIterator {
        scanner_core::iter::LookaheadIterator::new(self.lookaheads, self.index, self.end - self.index)
    }
}
