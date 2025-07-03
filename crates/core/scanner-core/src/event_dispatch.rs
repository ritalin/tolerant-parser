use engine_core::scanner_engine::{self, AcceptableRegexSet, ScanEvent};

#[derive(Clone)]
pub struct ScanEventDispatcher {
    source: String,
    index: usize,
    engine: scanner_engine::ScanningRuleSet,
}

impl ScanEventDispatcher {
    pub fn new(source: &str, index: usize, engine: scanner_engine::ScanningRuleSet) -> Self {
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

    pub fn next_regex(&mut self, regex_set: &AcceptableRegexSet) -> Option<ScanEvent> {
        match self.source.len().cmp(&self.index) {
            std::cmp::Ordering::Less => {
                None
            }
            std::cmp::Ordering::Equal => {
                match regex_set {
                    AcceptableRegexSet::Main => {
                        self.index += 1;
                        Some(ScanEvent { kind: self.engine.eof(), offset: self.source.len(), len: 0, value: None })
                    }
                    _ => None
                }
            }
            std::cmp::Ordering::Greater => {
                let event = self.engine.scan_by_regex(&self.source[self.index..], self.index, regex_set);
                if let Some(event) = event.as_ref() {
                    self.index += event.len;
                }
                event
            }
        }
    }

    pub fn next(&mut self, regex_set: &AcceptableRegexSet) -> Option<ScanEvent> {
        match self.source.len().cmp(&self.index) {
            std::cmp::Ordering::Less => {
                None
            }
            std::cmp::Ordering::Equal => {
                match regex_set {
                    AcceptableRegexSet::Main => {
                        self.index += 1;
                        Some(ScanEvent { kind: self.engine.eof(), offset: self.source.len(), len: 0, value: None })
                    }
                    _ => None
                }
            }
            std::cmp::Ordering::Greater => {
                let source = &self.source[self.index..];
                let event_1 = self.engine.scan_by_lexme(source, self.index);
                let event_2 = self.engine.scan_by_regex(source, self.index, regex_set);

                let event = match (event_1.as_ref(), event_2.as_ref()) {
                    (Some(lhs), Some(rhs)) if lhs.len < rhs.len => rhs,
                    (Some(lhs), Some(_)) => lhs,
                    (None, Some(rhs)) => rhs,
                    (Some(lhs), None) => lhs,
                    (None, None) => {
                        return None;
                    }
                };

                self.index += event.len;
                Some(event.clone())
            }
        }
    }

    pub fn invalid(&mut self) -> ScanEvent {
        let offset = self.index;
        let (len, value) = self.source.char_indices()
            .find_map(|(i, c)| if i == offset { Some(String::from(c)) } else { None } )
            .map(|v| (v.as_str().len(), v))
            .unzip()
        ;
        let len = len.unwrap_or_default();
        self.index += len;

        ScanEvent{ kind: self.engine.invalid(), offset, len, value }
    }

    pub fn has_more(&self) -> bool {
        self.index < self.source.len()
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

