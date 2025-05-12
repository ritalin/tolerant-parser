pub mod scanner_engine;
mod parser_engine;

/// Grammar symbol
#[derive(Debug, Clone)]
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
    pub symbol_rules: SymbolRuleSet,
    pub scanning_rules: scanner_engine::ScanningRuleSet,
    pub parsing_rules: ParsingRuleSet,
}

impl Default for Engine {
    fn default() -> Self {
        Self { 
            symbol_rules: Default::default(),
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

#[derive(Default)]
pub struct SymbolRuleSet;

impl SymbolRuleSet {
    pub fn syntax_kind_from_id(id: u32) -> SyntaxKind {
        todo!();
    }
}

#[derive(Default)]
pub struct ParsingRuleSet;
