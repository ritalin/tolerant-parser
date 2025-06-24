use std::collections::VecDeque;
use engine_core::SyntaxKind;

use crate::{event_dispatch::ScanEventDispatcher, Token};

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

pub struct StatementScanner {
    scan_range: std::ops::Range<usize>,
    lookaheads: VecDeque<Token>,
}

impl StatementScanner {
    pub fn as_view<'a, R: std::ops::RangeBounds<usize>>(&'a self, range: R) -> crate::scanner::StatementScannerView<'a> {
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

        crate::scanner::StatementScannerView::new(&self.lookaheads, from, len)
    }

    pub fn scan_range(&self) -> std::ops::Range<usize> {
        self.scan_range.clone()
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

pub struct StatementScannerIterator {
    lookaheads: VecDeque<Token>,
    emit_symbol: SyntaxKind,
    full_emit_symbol: Option<SyntaxKind>,
    dispatcher: ScanEventDispatcher,
}

impl StatementScannerIterator {
    pub fn new(lookaheads: VecDeque<Token>, dispatcher: ScanEventDispatcher, emit_symbol: SyntaxKind, full_emit_symbol: Option<SyntaxKind>) -> Self {
        Self { lookaheads, emit_symbol, full_emit_symbol, dispatcher }
    }
}

impl Iterator for StatementScannerIterator {
    type Item = StatementScanner;

    fn next(&mut self) -> Option<Self::Item> {
        let source_from = self.dispatcher.index();
        let size = crate::scanner::prefetch_internal(self.emit_symbol, &mut self.dispatcher, &mut self.lookaheads);
        if size == 0 {
            return None;
        }
        let source_to = self.dispatcher.index();

        match (self.lookaheads.front(), self.full_emit_symbol) {
            (Some(lookahead), Some(symbol)) if lookahead.main.kind == symbol => {
                // Drop Eof only statement scanner
                return None;
            }
            _ => {}
        }

        match self.lookaheads.len() == size {
            true => {
                Some(StatementScanner {
                    scan_range: source_from..source_to,
                    lookaheads: std::mem::take(&mut self.lookaheads),
                })
            }
            false => {
                Some(StatementScanner {
                    scan_range: source_from..source_to,
                    lookaheads: self.lookaheads.drain(0..size).collect()
                })
            }
        }
    }
}
