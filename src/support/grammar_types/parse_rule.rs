
#[derive(serde::Deserialize)]
pub struct GrammarParseRule {
    pub lhs: String,
    pub members: Vec<GrammarParseRuleMember>,
}

#[derive(serde::Deserialize)]
pub struct GrammarParseRuleMember {
    pub id: u32,
    pub sequences: Vec<Rhs>,
    pub precedence: Option<super::Precedence>,
}

#[derive(Clone, serde::Deserialize)]
pub struct Rhs(pub super::Term);

pub trait SymbolRef {
    fn id(&self) -> u32;
}

#[derive(Debug)]
pub struct RuleId {
    pub id: u32,
}

impl RuleId {
    pub fn new(id: u32) -> Self {
        Self { id }
    }
}

impl SymbolRef for RuleId {
    fn id(&self) -> u32 {
        self.id
    }
}
