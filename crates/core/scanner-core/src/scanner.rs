use std::collections::VecDeque;

use engine_core::scanner_engine;
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
        todo!()
    }

    pub fn save_scope(&self) -> ScannerScope {
        todo!()
    }

    pub fn restore_scope(&mut self, scope: ScannerScope) {
        todo!()
    }
}

pub struct ScannerScope {
    index: u32,
    lookaheads: Vec<Token>,
}

fn handle_scan_event(dispatcher: &mut ScanEventDispatcher) -> Option<Token> {
    todo!()
}
