use std::collections::{HashMap, HashSet};

use crate::SyntaxKind;

pub struct ScanningRuleSet {
    lexme_rule: fn(prefix: char) -> Option<&'static [ScanPattern]>,
    regex_rule: fn(id: u32) -> Option<&'static ScanPattern>,
    symbol_lookup: fn(id: u32) -> &'static crate::SyntaxKind,
    eof_id: u32,
}

impl ScanningRuleSet {
    pub fn new(
        lexme_rule: fn(prefix: char) -> Option<&'static [ScanPattern]>,
        regex_rule: fn(id: u32) -> Option<&'static ScanPattern>,
        symbol_lookup: fn(id: u32) -> &'static crate::SyntaxKind,
        eof_id: u32) -> Self
    {
        Self { lexme_rule, regex_rule, symbol_lookup, eof_id }
    }

    pub fn scan_by_lexme(&self, source: &str, offset: usize) -> Option<ScanEvent> {
        let Some(prefix) = source.chars().nth(0) else {
            return None;
        };
        let Some(patterns) = (self.lexme_rule)(prefix.to_ascii_lowercase()) else {
            return None;
        };

        for pattern in patterns {
            if source.starts_with(pattern.pattern) {
                let kind = (self.symbol_lookup)(pattern.id).clone();
                let value = source.get(0..pattern.len).map(String::from);
                return Some(ScanEvent{ kind, offset, len: pattern.len, value });
            }
        }
        
        None
    }

    pub fn scan_by_regex(&self, source: &str, offset: usize) -> Option<ScanEvent> {
        todo!()
    }

    pub fn eof(&self) -> SyntaxKind {
        (self.symbol_lookup)(self.eof_id).clone()
    }
}

impl Default for ScanningRuleSet {
    fn default() -> Self {
        Self { 
            lexme_rule: default_lexme_rule_lookup, 
            regex_rule: default_regex_rule_lookup,
            symbol_lookup: default_symbol_lookup,
            eof_id: 0,
        }
    }
}

static DEFAULT_SYNTAX_KIND: crate::SyntaxKind = crate::SyntaxKind { id: 0, text: "EOF", is_keyword: false, is_terminal: true };

fn default_lexme_rule_lookup(_sprefix: char) -> Option<&'static [ScanPattern]> {
    None
}

fn default_regex_rule_lookup(_id: u32) -> Option<&'static ScanPattern> {
    None
}

fn default_symbol_lookup(_id: u32) -> &'static crate::SyntaxKind {
    &DEFAULT_SYNTAX_KIND
}

#[derive(PartialEq, Debug)]
pub struct ScanEvent {
    /// Identifier for distinguishing token.
    pub kind: crate::SyntaxKind,
    /// potion (byte offset) from a document head
    pub offset: usize,
    /// token length
    pub len: usize,
    /// token value
    pub value: Option<String>
}

pub struct ScanPattern {
    pub id: u32,
    pub pattern: &'static str,
    pub len: usize,
}
