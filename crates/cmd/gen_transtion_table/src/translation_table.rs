use std::collections::{BTreeMap, HashMap, LinkedList};

use grammar_types_core::{parse_rule::{GrammarParseRule, GrammarParseRuleMember, RuleId}, scan_rule::GrammarCombinationSymbol, symbol::GrammarSymbol, SymbolType, Term};

use crate::configs::ActionResolveConfig;

pub struct ParseTableBuilder {
    config: ActionResolveConfig,
    grammar: lalry::Grammar<GrammarSymbolRef, String, RuleId>,
}

impl ParseTableBuilder {
    pub fn create(parse_rules: &[GrammarParseRule], symbols: &[GrammarSymbol], combination_symbols: &HashMap<String, GrammarCombinationSymbol>) -> Self {
        let symbol_lookup = create_symbol_lookup(symbols);

        let start_rule = parse_rules[0].lhs.clone();
        let mut lalry_rules = BTreeMap::new();
        let mut rule_id_gen = IdGenerator::new(parse_rules);

        create_lalry_rules(parse_rules, &symbol_lookup, combination_symbols, &mut rule_id_gen, &mut lalry_rules);
        append_combination_rules(combination_symbols, &symbol_lookup, &mut rule_id_gen, &mut lalry_rules);

        Self {
            config: ActionResolveConfig::new(parse_rules, symbols),
            grammar: lalry::Grammar { rules: lalry_rules, start: start_rule },
        }
    }

    pub fn build(&self) -> Result<lalry::LR1ParseTable<GrammarSymbolRef, String, RuleId>, anyhow::Error> {
        let parse_table = convert_to_lalr(&self.grammar, &self.config)?;

        Ok(parse_table)
    }
}

fn create_symbol_lookup(sources: &[GrammarSymbol]) -> HashMap<String, GrammarSymbolRef> {
    let mut symbols = HashMap::new();

    for source in sources {
        match source.symbol_type {
            SymbolType::Terminal { .. } => {
                symbols
                    .entry(source.name.clone())
                    .or_insert_with(|| GrammarSymbolRef(source.clone()));
            }
            SymbolType::NonTerminal => {}
            SymbolType::MultiTerminal { .. } => {
                symbols
                    .entry(source.name.clone())
                    .or_insert_with(|| GrammarSymbolRef(source.clone()));
            }
        }
    }

    symbols
}

fn create_lalry_rules(
    parse_rules: &[GrammarParseRule],
    symbol_lookup: &HashMap<String, GrammarSymbolRef>,
    combination_symbols: &HashMap<String, GrammarCombinationSymbol>,
    rule_id_gen: &mut IdGenerator,
    lalry_rules: &mut BTreeMap<String, Vec<lalry::Rhs<GrammarSymbolRef, String, RuleId>>> )
{
    for rule in parse_rules {
        let members = rule.members.iter()
            .flat_map(|member| {
                create_rhs_sequence(member, symbol_lookup, combination_symbols, rule_id_gen)
            })
            .map(|(id, seq)| lalry::Rhs { syms: seq, act: RuleId::new(id) })
            .collect::<Vec<_>>()
        ;

        lalry_rules.entry(rule.lhs.clone()).or_insert_with(|| members);
    }
}

fn create_rhs_sequence(
    rule: &GrammarParseRuleMember,
    symbol_lookup: &HashMap<String, GrammarSymbolRef>,
    combination_symbols: &HashMap<String, GrammarCombinationSymbol>,
    id_gen: &mut IdGenerator) -> Vec<(u32, Vec<lalry::Symbol<GrammarSymbolRef, String>>)> 
{
    let mut symbols = vec![];
    let mut symbol = vec![];
    id_gen.set_current(rule.id);

    create_symbols_internal(
        &rule.sequences,
        symbol_lookup,
        combination_symbols,
        id_gen,
        &mut symbol,
        &mut symbols,
    );

    symbols
}

fn create_symbols_internal(
    sequences: &[grammar_types_core::parse_rule::Rhs],
    symbol_lookup: &HashMap<String, GrammarSymbolRef>,
    combination_symbols: &HashMap<String, GrammarCombinationSymbol>,
    id_gen: &mut IdGenerator,
    current: &mut Vec<lalry::Symbol<GrammarSymbolRef, String>>,
    rhs_symbols: &mut Vec<(u32, Vec<lalry::Symbol<GrammarSymbolRef, String>>)>) 
{
    match sequences.get(0) {
        None => {
            rhs_symbols.push((id_gen.id(), current.clone()));
        }
        Some(grammar_types_core::parse_rule::Rhs(token)) => {
            // Try to resolve Reduce/Reduce conflict replacing by conbination rule
            if let Some((new_rule, rest)) = try_reduce_combination(token, &sequences[1..], combination_symbols) {
                current.push(lalry::Symbol::Nonterminal(new_rule.name));
                create_symbols_internal(&rest, symbol_lookup, combination_symbols, id_gen, current, rhs_symbols);
                return;
            }

            match token {
                Term::Symbol { name } if symbol_lookup.contains_key(name) => {
                    if let Some(GrammarSymbolRef(grammar_sym)) = symbol_lookup.get(name) {
                        match &grammar_sym.symbol_type {
                            SymbolType::Terminal { .. } | SymbolType::NonTerminal => {
                                current.push(lalry::Symbol::Terminal(GrammarSymbolRef(grammar_sym.clone())));
                                create_symbols_internal(&sequences[1..], symbol_lookup, combination_symbols, id_gen, current, rhs_symbols);
                            }
                            SymbolType::MultiTerminal { classes } => {
                                id_gen.flush();
                                for char_class in classes {
                                    if let Some(grammar_sym) = symbol_lookup.get(char_class) {
                                        let mut current = current.clone();
                                        current.push(lalry::Symbol::Terminal(grammar_sym.clone()));
                                        create_symbols_internal(&sequences[1..], symbol_lookup, combination_symbols, id_gen, &mut current, rhs_symbols);
                                    }
                                }
                            }
                        }
                    }
                }
                Term::Symbol { name } => {
                    current.push(lalry::Symbol::Nonterminal(name.clone()));
                    create_symbols_internal(&sequences[1..], symbol_lookup, combination_symbols, id_gen, current, rhs_symbols);
                }
                Term::CharClass { members } => {
                    id_gen.flush();
                    for term in members {
                        if let Some(grammar_sym) = symbol_lookup.get(term) {
                            let mut current = current.clone();
                            current.push(lalry::Symbol::Terminal(grammar_sym.clone()));
                            create_symbols_internal(&sequences[1..], symbol_lookup, combination_symbols, id_gen, &mut current, rhs_symbols);
                        }
                    }
                }
            }
        }
    }            
}

fn try_reduce_combination<'a>(
    term: &'a Term,
    sequences: &'a [grammar_types_core::parse_rule::Rhs],
    combination_symbols: &'a HashMap<String, GrammarCombinationSymbol>) -> Option<(GrammarSymbol, &'a [grammar_types_core::parse_rule::Rhs])> 
{
    if let Term::Symbol { name } = term.clone() {
        if let Some(GrammarCombinationSymbol { lhs, follow_symbols }) = combination_symbols.get(&name) {
            let matched =
                follow_symbols.iter().zip(sequences)
                .all(|(follow, grammar_types_core::parse_rule::Rhs(rhs))| match rhs {
                    Term::Symbol { name } if name.eq(follow) => true,
                    _ => false,
                })
            ;

            if matched {
                let new_rule = GrammarSymbol {
                    id: 0,
                    name: lhs.clone(),
                    symbol_type: SymbolType::NonTerminal,
                    precedence: None,
                };

                return Some((new_rule, &sequences[follow_symbols.len()..]));
            }
        }
    }

    None
}

fn append_combination_rules(
    combination_symbols: &HashMap<String, GrammarCombinationSymbol>,
    symbol_lookup: &HashMap<String, GrammarSymbolRef>,
    rule_id_gen: &mut IdGenerator,
    lalry_rules: &mut BTreeMap<String, Vec<lalry::Rhs<GrammarSymbolRef, String, RuleId>>>)
{
    for (first_symbol, GrammarCombinationSymbol { lhs, follow_symbols }) in combination_symbols {
        let id = rule_id_gen.id();

        let internal_syms = [first_symbol.clone()].iter()
            .chain(follow_symbols)
            .map(|name| match symbol_lookup.get(name) {
                Some(x) => lalry::Symbol::Terminal(x.clone()),
                None => lalry::Symbol::Nonterminal(name.clone()),
            })
            .collect::<Vec<_>>()
        ;

        lalry_rules.entry(lhs.clone()).or_insert_with(|| {
            vec![lalry::Rhs {
                syms: internal_syms,
                act: RuleId::new(id),
            }]
        });
    }
}

fn convert_to_lalr<'a>(
    grammar: &'a lalry::Grammar<GrammarSymbolRef, String, RuleId>,
    config: &'a ActionResolveConfig) -> Result<lalry::LR1ParseTable<'a, GrammarSymbolRef, String, RuleId>, anyhow::Error> 
{
    let table = grammar.lalr1(config).map_err(|conflict| {
        match conflict {
            lalry::LR1Conflict::ReduceReduce { state, token, r1: (name1, rhs1), r2: (name2, rhs2) } => {
                anyhow::anyhow!("Reduce/Reduce conflict (token: {:?}, rule1: {} := {:?}, rule2: {} := {:?}, state/len: {}", token, name1, rhs1, name2, rhs2, state.items.len())
            }
            lalry::LR1Conflict::ShiftReduce { state, token, rule: (name, rhs) } => {
                anyhow::anyhow!("Shift/Reduce conflict (token: {:?}), rule: {} := {:?}, state/len: {}", token, name, rhs, state.items.len())
            }
        }
    })?;

    Ok(table)
}

struct IdGenerator {
    stack: LinkedList<u32>,
    next: u32,
}

impl IdGenerator {
    pub fn new(rules: &[GrammarParseRule]) -> Self {
        let next_id = rules.iter()
            .map(|rule| rule.members.len() as u32)
            .sum::<u32>()
        ;
    
        Self {
            stack: LinkedList::new(),
            next: next_id,
        }
    }

    pub fn set_current(&mut self, id: u32) {
        self.stack.push_back(id);
    }
    pub fn flush(&mut self) {
        self.stack.clear();
    }
    pub fn id(&mut self) -> u32 {
        match self.stack.pop_back() {
            Some(id) => id,
            None => {
                let id = self.next;
                self.next += 1;
                id
            }
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct GrammarSymbolRef(pub GrammarSymbol);

impl grammar_types_core::parse_rule::SymbolRef for GrammarSymbolRef {
    fn id(&self) -> u32 {
        self.0.id
    }
}