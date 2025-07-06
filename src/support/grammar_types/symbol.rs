
#[derive(Eq, Ord, Clone, Debug, serde::Deserialize)]
pub struct GrammarSymbol {
    pub id: u32,
    pub name: String,
    #[serde(alias = "type")]
    pub symbol_type: super::SymbolType,
    pub precedence: Option<super::Precedence>,
}

impl PartialEq for GrammarSymbol {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd for GrammarSymbol {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}
