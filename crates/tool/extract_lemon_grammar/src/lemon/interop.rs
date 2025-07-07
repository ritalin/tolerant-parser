use std::ffi::CString;

use tolerant_parser_sdk::support::grammar_types::SymbolType;

use super::lemon_bindings;

pub unsafe fn rule_raw_by_name(lemon: *mut lemon_bindings::lemon, needle: &str) -> *mut lemon_bindings::rule {
    unsafe {
        let mut rule = (*lemon).rule;

        while ! rule.is_null() {
            let lhs = super::Rule::name_from(rule);
            if lhs == needle {
                return rule;
            }
            rule = (*rule).next;
        }

        std::ptr::null_mut()
    }
}

pub unsafe fn expand_rhs_symbol(rule: *mut lemon_bindings::rule, symbols: &[*mut lemon_bindings::symbol]) {
    unsafe {
        let new_size = ((*rule).nrhs as usize) + symbols.len();

        let new_layout = std::alloc::Layout::array::<*mut lemon_bindings::symbol>(new_size).expect("New layout failed");

        let new_rules = std::alloc::alloc(new_layout) as *mut *mut lemon_bindings::symbol;

        let offset = (*rule).nrhs as usize;
        for i in 0..offset {
            *new_rules.add(i) = *(*rule).rhs.add(i);
        }
        for i in 0..symbols.len() {
            *new_rules.add(i + offset) = symbols[i];
        }
        (*rule).rhs = new_rules;
        (*rule).nrhs = new_size as i32;
    }
}

pub fn enumerate_symbols(symbols: *mut *mut lemon_bindings::symbol, len: usize) -> impl Iterator<Item = super::Symbol> {
    unsafe { std::slice::from_raw_parts(symbols, len) }.into_iter()
    .enumerate()
    .map(|(i, &x)| super::Symbol::from_raw(i + 1, x))
}

pub fn enumerate_rules(mut rule_raw: *mut lemon_bindings::rule) -> Vec<super::Rule> {
    let mut rule_map = std::collections::BTreeMap::<i32, Vec<*mut lemon_bindings::rule>>::new();

    while ! rule_raw.is_null() {
        unsafe {
            let mut next_lhs = (*rule_raw).nextlhs;
            let mut index = (*rule_raw).index;

            while !next_lhs.is_null() {
                index = (*next_lhs).index;
                next_lhs = (*next_lhs).nextlhs;
            }
            rule_map.entry(index)
                .and_modify(|members| members.push(rule_raw))
                .or_insert_with(|| vec![rule_raw])
            ;

            rule_raw = (*rule_raw ).next;
        }
    }

    rule_map.values().into_iter()
        .map(|members| super::Rule::from_raw(&members))
        .collect::<Vec<_>>()
}

pub(crate) fn symbol_type_from_raw(symbol: *mut lemon_bindings::symbol, symbol_name: &str) -> super::SymbolType {
    match (unsafe { *symbol }).type_ {
        lemon_bindings::symbol_type_TERMINAL => {
            let name = symbol_name.to_lowercase();
            let name_len = name.len();

            let is_keyword = unsafe { super::keyword_check::sqlite3_keyword_check(CString::new(name).unwrap().as_ptr(), name_len as i32) };
            SymbolType::Terminal { is_keyword: is_keyword == 1 }
        }
        lemon_bindings::symbol_type_NONTERMINAL => {
            SymbolType::NonTerminal
        }
        lemon_bindings::symbol_type_MULTITERMINAL => {
            let classes = multi_terminal_member_names(symbol);
            SymbolType::MultiTerminal{ classes }
        }
        n => panic!("unexpected symbol type value ({n})"),
    }
}

pub(crate) fn multi_terminal_member_names(symbol: *mut lemon_bindings::symbol) -> Vec<String> {
    unsafe { std::slice::from_raw_parts((*symbol).subsym, (*symbol).nsubsym as usize) }.into_iter()
    .map(|&sub| super::Symbol::from_raw(0, sub).name())
    .collect()
}

pub(crate) fn precedence_from_raw(sym: *mut lemon_bindings::symbol) -> Option<super::Precedence> {
    match unsafe { ((*sym).assoc, (*sym).prec) } {
        (lemon_bindings::e_assoc_LEFT, prec) => Some(super::Precedence::Left(prec)),
        (lemon_bindings::e_assoc_RIGHT, prec) => Some(super::Precedence::Right(prec)),
        (lemon_bindings::e_assoc_NONE, _) => Some(super::Precedence::Noassoc),
        (lemon_bindings::e_assoc_UNK, _) => None,
        (assoc, prec) => panic!("Unexpected precedence value (assoc: {assoc}, prec: {prec})"),
    }
}