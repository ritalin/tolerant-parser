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
    source_from: usize,
    lookaheads: VecDeque<Token>,
}

impl StatementScanner {
    pub fn as_view<'a, R: std::ops::RangeBounds<usize>>(&'a self, range: R) -> crate::scanner::StatementScannerView<'a> {
        let (start, end) = match (range.start_bound(), range.end_bound()) {
            (std::ops::Bound::Included(s), std::ops::Bound::Included(e)) => (*s, *e+1),
            (std::ops::Bound::Included(s), std::ops::Bound::Excluded(e)) => (*s, *e),
            (std::ops::Bound::Included(s), std::ops::Bound::Unbounded) => (*s, usize::MAX),
            (std::ops::Bound::Excluded(s), std::ops::Bound::Included(e)) => (*s+1, *e+1),
            (std::ops::Bound::Excluded(s), std::ops::Bound::Excluded(e)) => (*s+1, *e),
            (std::ops::Bound::Excluded(s), std::ops::Bound::Unbounded) => (*s+1, usize::MAX),
            (std::ops::Bound::Unbounded, std::ops::Bound::Included(e)) => (self.source_from, *e),
            (std::ops::Bound::Unbounded, std::ops::Bound::Excluded(e)) => (self.source_from, *e+1),
            (std::ops::Bound::Unbounded, std::ops::Bound::Unbounded) => (self.source_from, usize::MAX),
        };

        let (from, to) = self.lookaheads.iter().enumerate()
            .skip_while(|(_, la)| la.highest_offset() <= start)
            .take_while(|(_, la)| la.lowest_offset() < end)
            .fold((usize::MAX, 0), |(from, to), (i, _)| {
                (usize::min(i, from), usize::max(i, to))
            })
        ;
        let (from, len) = if from == usize::MAX { (0, 0) } else { (from, to - from + 1) };

        crate::scanner::StatementScannerView::new(&self.lookaheads, from, len)
    }
}

pub struct StatementScannerIterator {
    lookaheads: VecDeque<Token>,
    terminate_symbol: SyntaxKind,
    dispatcher: ScanEventDispatcher,
}

impl StatementScannerIterator {
    pub fn new(lookaheads: VecDeque<Token>, dispatcher: ScanEventDispatcher, terminate_symbol: SyntaxKind) -> Self {
        Self { lookaheads, terminate_symbol, dispatcher }
    }
}

impl Iterator for StatementScannerIterator {
    type Item = StatementScanner;

    fn next(&mut self) -> Option<Self::Item> {
        let source_from = self.dispatcher.index();
        let size = crate::scanner::prefetch_internal(self.terminate_symbol, &mut self.dispatcher, &mut self.lookaheads);
        if size == 0 {
            return None;
        }

        match self.lookaheads.len() == size {
            true => {
                Some(StatementScanner {
                    source_from,
                    lookaheads: std::mem::take(&mut self.lookaheads),
                })
            }
            false => {
                Some(StatementScanner {
                    source_from,
                    lookaheads: self.lookaheads.drain(0..size).collect()
                })
            }
        }
    }
}
