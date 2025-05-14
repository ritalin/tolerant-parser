pub mod scan_rule;
pub mod parse_rule;
pub mod symbol;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum SymbolType {
    Terminal{ is_keyword: bool },
    NonTerminal,
    MultiTerminal{ classes: Vec<String>},
}

#[derive(PartialEq, Eq, Ord, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Precedence {
    Left(i32),
    Right(i32),
    Noassoc,
}

impl Precedence {
    pub fn score(&self) -> i32 {
        match self {
            Precedence::Left(score) => *score,
            Precedence::Right(score) => *score,
            Precedence::Noassoc => 0
        }
    }
}

impl PartialOrd for Precedence {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let lhs_score = match self.score() {
            score if score > 0 => Some(score),
            _ => None
        };
        let rhs_score = match other.score(){
            score if score > 0 => Some(score),
            _ => None
        };

        rhs_score.partial_cmp(&lhs_score)
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Term {
    Symbol {name: String},
    CharClass { members: Vec<String> },
}
