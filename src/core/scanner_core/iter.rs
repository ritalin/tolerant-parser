use std::collections::VecDeque;
use crate::core::engine_core::SyntaxKind;

use crate::core::scanner_core::StatementScannerView;
use crate::core::scanner_core::{event_dispatch::ScanEventDispatcher, Token};
use crate::core::scanner_core::scanner;

#[derive(PartialEq, Clone, Debug)]
pub struct LookaheadIterator<'a> {
    inner: &'a VecDeque<Token>,
    index: usize,
    start: usize,
    end: usize,
}

impl<'a> LookaheadIterator<'a> {
    pub fn new(lookaheads: &'a VecDeque<Token>, index: usize, size: usize) -> Self {
        Self {
            inner: lookaheads,
            index,
            start: index,
            end: index + size,
        }
    }

    pub fn peek(&self) -> Option<&'a Token> {
        self.inner.get(self.index)
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }
}

impl<'a> Iterator for LookaheadIterator<'a> {
    type Item = &'a Token;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.end {
            return None;
        }

        let token = self.inner.get(self.index);
        self.index += 1;

        token
    }
}

#[derive(PartialEq, Debug)]
pub struct StatementScanner {
    scanner_type: StatementScannerType,
    scan_range: std::ops::Range<usize>,
    is_full_emit: bool,
    lookaheads: VecDeque<Token>,
}

impl StatementScanner {
    pub fn as_view<'a, R: std::ops::RangeBounds<usize>>(&'a self, range: R) -> StatementScannerView<'a> {
        let (start, end) = match (range.start_bound(), range.end_bound()) {
            (std::ops::Bound::Included(s), std::ops::Bound::Included(e)) => (*s, *e+1),
            (std::ops::Bound::Included(s), std::ops::Bound::Excluded(e)) => (*s, *e),
            (std::ops::Bound::Included(s), std::ops::Bound::Unbounded) => (*s, self.scan_range.end),
            (std::ops::Bound::Excluded(s), std::ops::Bound::Included(e)) => (*s+1, *e+1),
            (std::ops::Bound::Excluded(s), std::ops::Bound::Excluded(e)) => (*s+1, *e),
            (std::ops::Bound::Excluded(s), std::ops::Bound::Unbounded) => (*s+1, self.scan_range.end),
            (std::ops::Bound::Unbounded, std::ops::Bound::Included(e)) => (self.scan_range.start, *e),
            (std::ops::Bound::Unbounded, std::ops::Bound::Excluded(e)) => (self.scan_range.start, *e+1),
            (std::ops::Bound::Unbounded, std::ops::Bound::Unbounded) => (self.scan_range.start, self.scan_range.end),
        };

        let (from, to) = self.lookaheads.iter()
            .map(|la| (la.lowest_offset(), la.token_len()))
            .enumerate()
            .skip_while(|(_, (lowest, token_len))| match (lowest + token_len).cmp(&start) {
                std::cmp::Ordering::Equal if *token_len == 0 => false,
                std::cmp::Ordering::Greater => false,
                _ => true,
            })
            .take_while(|(_, (lowest, _))| *lowest < end)
            .fold((usize::MAX, 0), |(from, to), (i, _)| {
                (usize::min(i, from), usize::max(i, to))
            })
        ;

        let (from, len) = if from == usize::MAX { (0, 0) } else { (from, to - from + 1) };

        StatementScannerView::new(&self.lookaheads, from, if self.is_full_emit { len + 1 } else { len })
    }

    pub fn scan_range(&self) -> std::ops::Range<usize> {
        self.scan_range.clone()
    }

    pub fn scanner_type(&self) -> StatementScannerType {
        self.scanner_type.clone()
    }
}

impl std::fmt::Display for StatementScanner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let iter = self.lookaheads.iter()
            .flat_map(|la| vec![ la.leading_trivia.clone(), Some(vec![la.main.clone()]), la.trailing_trivia.clone() ])
            .flatten()
            .flatten()
            .filter_map(|x| x.value)
        ;

        for value in iter {
            write!(f, "{value}")?;
        }
        
        Ok(())
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum StatementScannerType { Statement, Eof }

pub struct StatementScannerIterator {
    lookaheads: VecDeque<Token>,
    emit_symbol: SyntaxKind,
    full_emit_symbol: SyntaxKind,
    next_source_from: usize,
    dispatcher: ScanEventDispatcher,
}

impl StatementScannerIterator {
    pub fn new(lookaheads: VecDeque<Token>, dispatcher: ScanEventDispatcher, emit_symbol: SyntaxKind, full_emit_symbol: SyntaxKind) -> Self {
        Self { lookaheads, emit_symbol, full_emit_symbol, next_source_from: dispatcher.index(), dispatcher }
    }
}

impl Iterator for StatementScannerIterator {
    type Item = StatementScanner;

    fn next(&mut self) -> Option<Self::Item> {
        let source_from = self.next_source_from;
        let size = scanner::prefetch_internal(self.emit_symbol, &mut self.dispatcher, &mut self.lookaheads);
        if size == 0 {
            return None;
        }
        let mut is_full_emit = false;
        self.next_source_from = self.dispatcher.index();

        if let Some(lookahead) = self.lookaheads.back() {
             if (size > 1) && (lookahead.main.kind == self.full_emit_symbol) {
                // Prefetch is finished without emitting the statement
                // So To create Eof statement, push back the full emit token to lookahead cache
                let token_len = lookahead.token_len();
                self.lookaheads.push_back(lookahead.clone());
                self.next_source_from -= token_len + 1;
                is_full_emit = true;
            }
        }

        match (self.lookaheads.len() == size, self.lookaheads.front()) {
            (true, Some(lookahead)) if lookahead.main.kind == self.full_emit_symbol => {
                // Eof only statement scanner
                Some(StatementScanner {
                    scanner_type: StatementScannerType::Eof,
                    scan_range: source_from..(source_from + lookahead.token_len()),
                    is_full_emit: true,
                    lookaheads: std::mem::take(&mut self.lookaheads)
                })
            }
            (true, _) => {
                Some(StatementScanner {
                    scanner_type: StatementScannerType::Statement,
                    scan_range: source_from..self.next_source_from,
                    is_full_emit,
                    lookaheads: std::mem::take(&mut self.lookaheads),
                })
            }
            (false, _) => {
                Some(StatementScanner {
                    scanner_type: StatementScannerType::Statement,
                    scan_range: source_from..self.next_source_from,
                    is_full_emit,
                    lookaheads: self.lookaheads.drain(0..size).collect()
                })
            }
        }
    }
}

pub struct CachedStatementScannerIterator {
    lookaheads: VecDeque<Token>,
    emit_symbol: SyntaxKind,
    full_emit_symbol: SyntaxKind,
}

impl CachedStatementScannerIterator {
    pub fn new<I>(lookaheads: I, emit_symbol: SyntaxKind, full_emit_symbol: SyntaxKind) -> Self 
    where I: IntoIterator<Item = Token>
    {
        Self {
            lookaheads: VecDeque::from_iter(lookaheads.into_iter()),
            emit_symbol,
            full_emit_symbol,
        }
    }
}

impl Iterator for CachedStatementScannerIterator {
    type Item = StatementScanner;

    fn next(&mut self) -> Option<Self::Item> {
        if self.lookaheads.is_empty() { return  None }

        match self.lookaheads.iter().position(|la| la.main.kind == self.emit_symbol) {
            Some(index) => {
                let (Some(head), Some(tail)) = (self.lookaheads.get(0), self.lookaheads.get(index)) else { return None };

                Some(StatementScanner {
                    scanner_type: StatementScannerType::Statement,
                    scan_range: (head.lowest_offset())..(tail.token_range().end),
                    is_full_emit: tail.main.kind == self.full_emit_symbol,
                    lookaheads: self.lookaheads.drain(0..=index).collect()
                })
            }
            None => {
                let Some(head) = self.lookaheads.get(0) else { return None };

                Some(StatementScanner {
                    scanner_type: StatementScannerType::Eof,
                    scan_range: head.token_range(),
                    is_full_emit: true,
                    lookaheads: self.lookaheads.drain(..).collect(),
                })
            }
        }
    }
}