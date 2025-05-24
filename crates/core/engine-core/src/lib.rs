pub mod scanner_engine;
pub mod parser_engine;

pub use scanner_engine::default_syntax_kind;

/// Grammar symbol
#[derive(Eq, Ord, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct SyntaxKind {
    pub id: u32,
    pub text: &'static str,
    pub group: SymbolGroup,
}

impl PartialEq for SyntaxKind {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd for SyntaxKind {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl std::hash::Hash for SyntaxKind {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum SymbolGroup {
    /// Terminal symbol of keyword scanned by lexme
    Keyword,
    /// Terminal symbol of non keyword scanned by lexme
    NonKeyword,
    /// Terminal symbol scanned by regex
    Pattern,
    /// Non-Terminal symbol
    NonTerminal,
}

#[derive(Clone)]
pub struct Engine {
    pub scanning_rules: scanner_engine::ScanningRuleSet,
    pub parsing_rules: parser_engine::ParsingRuleSet,
}

impl Default for Engine {
    fn default() -> Self {
        Self { 
            scanning_rules: Default::default(), 
            parsing_rules: Default::default() 
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("Can not initialize scanner engine (cause: {0})")]
    ScanningRuleCreateFailed(String),
    #[error("Can not initialize parser engine (cause: {0})")]
    PrsingRuleCreateFailed(String)
}
