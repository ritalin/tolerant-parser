use std::arch::x86_64::_MM_ROUND_DOWN;

use engine_core::scanner_engine::{self, ScanEvent};


pub struct ScanEventDispatcher {
    source: String,
    index: usize,
    engine: scanner_engine::ScanningRuleSet,
}

impl ScanEventDispatcher {
    pub fn new(source: &str, index: u32, engine: scanner_engine::ScanningRuleSet) -> Self {
        Self { source: source.into(), index: index as usize, engine }
    }

    pub fn next_lexme(&mut self) -> Option<ScanEvent> {
        match self.source.len().cmp(&self.index) {
            std::cmp::Ordering::Less => {
                None
            }
            std::cmp::Ordering::Equal => {
                self.index += 1;
                Some(ScanEvent { kind: self.engine.eof(), offset: self.source.len(), len: 0, value: None })
            }
            std::cmp::Ordering::Greater => {
                let event = self.engine.scan_by_lexme(&self.source[self.index..], self.index);
                if let Some(event) = event.as_ref() {
                    self.index += event.len;
                }
                event
            }
        }
    }

    pub fn next_regex(&mut self) -> Option<ScanEvent> {
        match self.source.len().cmp(&self.index) {
            std::cmp::Ordering::Greater => {
                None
            }
            std::cmp::Ordering::Equal => {
                self.index += 1;
                Some(ScanEvent { kind: self.engine.eof(), offset: self.source.len(), len: 0, value: None })
            }
            std::cmp::Ordering::Less => {
                todo!()
            }
        }
    }

    pub fn next(&mut self) -> Option<ScanEvent> {
        todo!()
    }
}

