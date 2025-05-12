use std::collections::{BTreeMap, HashMap};

#[derive(serde::Deserialize)]
pub struct GrammarScanRule {
    pub lexme: HashMap<String, Vec<String>>,
    pub regex: BTreeMap<String, Vec<RegexGrammarScanRule>>,
    pub alternatives: HashMap<String, Vec<String>>,
}

#[derive(serde::Deserialize)]
pub struct RegexGrammarScanRule {
    pub pattern: String,
    #[serde(default)]
    pub leading: bool,
    #[serde(default)]
    pub trailing: bool,
    #[serde(default)]
    pub main: bool,
}
