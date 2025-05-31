pub mod event_dispatch;

mod scanner;
pub use scanner::Scanner;
pub mod iter;

use engine_core::scanner_engine::ScanEvent;

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
}

#[derive(Debug, thiserror::Error)]
pub enum ScannerError {
    #[error("Can not create scanner instance")]
    CreateFailed,
}
