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
    lookaheads: VecDeque<Token>,
}

impl StatementScanner {
    pub fn as_view<'a>(&'a self, range: std::ops::Range<usize>) -> crate::scanner::StatementScannerView<'a> {
        let (from, to) = self.lookaheads.iter().enumerate()
            .skip_while(|(_, la)| la.highest_offset() <= range.start)
            .take_while(|(_, la)| la.lowest_offset() < range.end)
            .fold((usize::MAX, 0), |(from, to), (i, _)| {
                (usize::min(i, from), usize::max(i, to))
            })
        ;
        let len = if from == usize::MAX { 0 } else { to - from + 1 };

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
        let size = crate::scanner::prefetch_internal(self.terminate_symbol, &mut self.dispatcher, &mut self.lookaheads);
        if size == 0 {
            return None;
        }

        match self.lookaheads.len() == size {
            true => {
                Some(StatementScanner {
                    lookaheads: std::mem::take(&mut self.lookaheads),
                })
            }
            false => {
                Some(StatementScanner {
                    lookaheads: self.lookaheads.drain(0..size).collect()
                })
            }
        }
    }
}
