pub mod dispatch;

mod scanner;
pub use scanner::Scanner;

use engine_core::scanner_engine::ScanEvent;

pub struct Token {
    /// Leading trivia is containing comments, white space and so on.
    pub leading_trivia: Option<Vec<ScanEvent>>,
    /// Focused main token.
    pub main: ScanEvent,
    /// Trailing trivia is containing white space and so on.
    pub trailing_trivia: Option<Vec<ScanEvent>>,
}

#[derive(Debug, thiserror::Error)]
pub enum ScannerError {
    #[error("Can not create scanner instance")]
    CreateFailed,
}
