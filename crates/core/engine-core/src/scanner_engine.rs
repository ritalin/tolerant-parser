use std::{collections::HashMap, rc::Rc};

use crate::SyntaxKind;

#[derive(Clone)]
pub struct ScanningRuleSet {
    lexme_rule: fn(prefix: char) -> Option<&'static [ScanPattern]>,
    acceptable_regex: fn(regex_set: &AcceptableRegexSet) -> Option<&'static [usize]>,
    symbol_lookup: fn(id: u32) -> &'static crate::SyntaxKind,
    eof_id: u32,
    regex_cache: Rc<HashMap<usize, RegexScanPattern>>,
}

impl ScanningRuleSet {
    pub fn new(
        lexme_rule: fn(prefix: char) -> Option<&'static [ScanPattern]>,
        regex_rule: fn(index: usize) -> Option<&'static ScanPattern>,
        acceptable_regex: fn(regex_set: &AcceptableRegexSet) -> Option<&'static [usize]>,
        symbol_lookup: fn(id: u32) -> &'static crate::SyntaxKind,
        eof_id: u32) -> Self
    {
        let regex_cache = init_regex_cache(regex_rule, acceptable_regex);

        Self { 
            lexme_rule, acceptable_regex, symbol_lookup, eof_id,
            regex_cache: Rc::new(regex_cache),
        }
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

    pub fn scan_by_regex(&self, source: &str, offset: usize, regex_set: &AcceptableRegexSet) -> Option<ScanEvent> {
        let Some(regex_indexes) = (self.acceptable_regex)(regex_set) else {
            return None;
        };
        
        for pattern in regex_indexes.iter().filter_map(|i| self.regex_cache.get(i)) {
            match pattern.pattern.find_at(source, 0) {
                Some(m) if m.start() == 0 => {
                    let kind = (self.symbol_lookup)(pattern.id).clone();
                    let value = Some(m.as_str().to_string());
                    return Some(ScanEvent { kind, offset, len: m.len(), value });
                }
                _ => {}
            }
        }
        
        None
    }

    pub fn eof(&self) -> SyntaxKind {
        (self.symbol_lookup)(self.eof_id).clone()
    }
}

fn init_regex_cache(
    regex_rule: fn(index: usize) -> Option<&'static ScanPattern>,
    acceptable_regex: fn(regex_set: &AcceptableRegexSet) -> Option<&'static [usize]>) -> HashMap<usize, RegexScanPattern> 
{
    let mut cache = HashMap::new();

    init_regex_cache_internal(regex_rule, acceptable_regex, AcceptableRegexSet::Leading, &mut cache);
    init_regex_cache_internal(regex_rule, acceptable_regex, AcceptableRegexSet::Main, &mut cache);
    init_regex_cache_internal(regex_rule, acceptable_regex, AcceptableRegexSet::Trailing, &mut cache);

    cache
}

fn init_regex_cache_internal(
    regex_rule: fn(index: usize) -> Option<&'static ScanPattern>,
    acceptable_regex: fn(regex_set: &AcceptableRegexSet) -> Option<&'static [usize]>,
    regex_set: AcceptableRegexSet, 
    cache: &mut HashMap<usize, RegexScanPattern>)
{
    let Some(regex_indexes) = (acceptable_regex)(&regex_set) else {
        return;
    };

    for i in regex_indexes {
        match cache.entry(*i) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                if let Some(pattern) = (regex_rule)(*i) {
                    entry.insert(RegexScanPattern { 
                        id: pattern.id, 
                        pattern: regex::Regex::new(pattern.pattern).expect(&format!("Can not instantiate regex scan pattern (pattern: `{}`)", pattern.pattern))
                    });
                }
            }
            std::collections::hash_map::Entry::Occupied(_) => {}
        }
    }
}

impl Default for ScanningRuleSet {
    fn default() -> Self {
        Self { 
            lexme_rule: default_lexme_rule_lookup, 
            acceptable_regex: default_acceptable_regex_lookup,
            symbol_lookup: default_symbol_lookup,
            eof_id: 0,
            regex_cache: Default::default(),
        }
    }
}

pub mod default_syntax_kind {
    use crate::SymbolGroup;

    pub static DEFAULT: crate::SyntaxKind = crate::SyntaxKind { id: 0, text: "EOF", group: SymbolGroup::NonKeyword };
}

fn default_lexme_rule_lookup(_sprefix: char) -> Option<&'static [ScanPattern]> {
    None
}

fn default_acceptable_regex_lookup(_regex_set: &AcceptableRegexSet) -> Option<&'static [usize]> {
    None
}

pub fn default_symbol_lookup(_id: u32) -> &'static crate::SyntaxKind {
    &default_syntax_kind::DEFAULT
}

#[derive(PartialEq, Clone, Debug)]
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

pub struct RegexScanPattern {
    pub id: u32,
    pub pattern: regex::Regex,
}

#[derive(Hash)]
pub enum AcceptableRegexSet {
    /// requast leading token patterns
    Leading,
    /// request main token patterns
    Main,
    /// request trailing token patterns
    Trailing,
}
