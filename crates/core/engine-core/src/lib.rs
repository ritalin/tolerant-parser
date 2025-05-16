pub mod scanner_engine;
pub mod parser_engine;

pub use scanner_engine::default_syntax_kind;

/// Grammar symbol
#[derive(Eq, Ord, Debug, Clone, Copy)]
pub struct SyntaxKind {
    pub id: u32,
    pub text: &'static str,
    pub is_keyword: bool,
    pub is_terminal: bool,
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
    #[error("Can not initialize parser/scanner engine")]
    CreateFailed
}
