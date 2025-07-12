pub mod event_dispatch;

mod scanner;
pub use scanner::{Scanner, StatementScannerView, ScannerAccess, ScannerOption};
pub mod iter;

use super::engine_core::scanner_engine::ScanEvent;

#[derive(PartialEq, Clone, Debug)]
pub struct Token {
    /// Leading trivia is containing comments, white space and so on.
    pub leading_trivia: Option<Vec<ScanEvent>>,
    /// Focused main token.
    pub main: ScanEvent,
    /// Trailing trivia is containing white space and so on.
    pub trailing_trivia: Option<Vec<ScanEvent>>,
}

impl Token {
    pub fn lowest_offset(&self) -> usize {
        self.leading_trivia.as_ref()
        .and_then(|xs| xs.first())
        .map(|x| x.offset)
        .unwrap_or_else(|| self.main.offset)
    }

    pub fn highest_offset(&self) -> usize {
        self.trailing_trivia.as_ref()
        .and_then(|xs| xs.last())
        .map(|x| x.offset + x.len)
        .unwrap_or_else(|| self.main.offset + self.main.len)
    }

    pub fn token_len(&self) -> usize {
        let leading_trivia_len = self.leading_trivia.as_ref()
            .map(|xs| xs.iter().map(|x| x.len).sum::<usize>())
            .unwrap_or_default()
        ;
        let trailing_trivia_len = self.trailing_trivia.as_ref()
            .map(|xs| xs.iter().map(|x| x.len).sum::<usize>())
            .unwrap_or_default()
        ;

        leading_trivia_len + self.main.len + trailing_trivia_len
    }

    pub fn token_range(&self) -> std::ops::Range<usize> {
        let start = self.lowest_offset();
        let len = self.token_len();

        start..(start + len)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ScannerError {
    #[error("Can not create scanner instance")]
    CreateFailed,
}
