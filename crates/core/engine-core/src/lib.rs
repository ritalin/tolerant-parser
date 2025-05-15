pub mod scanner_engine;
pub mod parser_engine;

pub use scanner_engine::default_syntax_kind;

/// Grammar symbol
#[derive(Debug, Clone, Copy)]
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
