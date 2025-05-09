use std::{ffi::{CStr, CString}, mem::MaybeUninit};
use grammar_types_core::{Precedence, Term};

pub mod keyword_check;
pub mod lemon_bindings;

use keyword_check::sqlite3_keyword_check;

pub struct LemonBuilder {
    inner: lemon_bindings::lemon,
}

impl LemonBuilder {
    pub fn new() -> Self {
        let mut inner: lemon_bindings::lemon = {
            let lem = MaybeUninit::zeroed();
            unsafe { lem.assume_init() }
        };
        inner.errorcnt = 0;
        inner.basisflag = 0;
        inner.nolinenosflag = 0;

        Self { inner }
    }

    pub fn set_args(mut self, args: std::env::Args) -> Self {
        let args = args.map(|arg| CString::new(arg).unwrap())
            .collect::<Vec<_>>()
        ;
        let mut c_args: Vec<*mut i8> = args.iter()
            .map(|arg| arg.as_ptr() as *mut i8)
            .collect()
        ;
        c_args.push(std::ptr::null_mut());

        self.inner.argc = c_args.len() as i32;
        self.inner.argv = c_args.as_mut_ptr();

        self
    }

    pub fn set_grammar(mut self, grammar_path: &str) -> Self {
        self.inner.filename = grammar_path.as_ptr() as *mut i8;

        self
    }

    pub fn build(self) -> Lemon {
        Lemon::from_raw(self.inner)
    }
}

pub struct Lemon {
    inner: lemon_bindings::lemon,
}

impl Lemon {
    pub fn from_raw(ptr: lemon_bindings::lemon) -> Self {
        Self { inner: ptr }
    }

    pub fn parse(&mut self) {
        unsafe { 
            lemon_bindings::Symbol_init();
            lemon_bindings::Parse(&mut self.inner);
        
            let sym_eof = lemon_bindings::Symbol_new(CString::new("EOF").unwrap().as_ptr());

            let start_rule = rule_raw_by_name(&mut self.inner, "input");
            expand_rhs_symbol(start_rule, &[sym_eof]);

            self.inner.nsymbol = lemon_bindings::Symbol_count();
            self.inner.symbols = lemon_bindings::Symbol_arrayof();
        };

    }

    pub fn symbols(&self) -> Vec<Symbol> {
        symbols_internal(self.inner.symbols, self.inner.nsymbol as usize).collect()
    }

    pub fn rules(&self) -> Vec<Rule> {
        let mut rule_map = std::collections::BTreeMap::<i32, Vec<*mut lemon_bindings::rule>>::new();
        let mut rule_raw = self.inner.rule;

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
            .map(|members| Rule::from_raw(&members))
            .collect::<Vec<_>>()
    }
}

unsafe fn rule_raw_by_name(lemon: *mut lemon_bindings::lemon, needle: &str) -> *mut lemon_bindings::rule {
    unsafe {
        let mut rule = (*lemon).rule;

        while ! rule.is_null() {
            let lhs = Rule::name_from(rule);
            if lhs == needle {
                return rule;
            }
            rule = (*rule).next;
        }

        std::ptr::null_mut()
    }
}

unsafe fn expand_rhs_symbol(rule: *mut lemon_bindings::rule, symbols: &[*mut lemon_bindings::symbol]) {
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

fn symbols_internal(symbols: *mut *mut lemon_bindings::symbol, len: usize) -> impl Iterator<Item = Symbol> {
    unsafe { std::slice::from_raw_parts(symbols, len) }.into_iter()
    .enumerate()
    .map(|(i, &x)| Symbol::from_raw(i + 1, x))
}

pub struct Symbol {
    id: usize,
    inner: *mut lemon_bindings::symbol,
}

impl Symbol {
    fn from_raw(id: usize, inner: *mut lemon_bindings::symbol) -> Self {
        Self { id, inner }
    }

    pub fn name(&self) -> String {
        Symbol::symbol_name(self.inner)
    }

    pub fn symbol_name(symbol: *mut lemon_bindings::symbol) -> String {
        unsafe { CStr::from_ptr((*symbol).name)
            .to_string_lossy()
            .to_string() 
        } 
    }

    pub fn symbol_type(&self) -> SymbolType {
        SymbolType::from_raw(self.inner, &self.name())
    }

    pub fn precedence(&self) -> Option<Precedence> {
        precedence_from_raw(self.inner)
    }
}

impl serde::Serialize for Symbol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer 
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("symbol", 4)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("name", &self.name())?;
        state.serialize_field("type", &self.symbol_type())?;
        state.serialize_field("precedence", &self.precedence())?;
        state.end()
    }
}

#[derive(serde::Serialize)]
pub enum SymbolType {
    Terminal{ is_keyword: bool },
    NonTerminal,
    MultiTerminal{ classes: Vec<String>},
}

impl SymbolType {
    fn from_raw(symbol: *mut lemon_bindings::symbol, symbol_name: &str) -> Self {
        match (unsafe { *symbol }).type_ {
            lemon_bindings::symbol_type_TERMINAL => {
                let name = symbol_name.to_lowercase();
                let name_len = name.len();

                let is_keyword = unsafe { sqlite3_keyword_check(CString::new(name).unwrap().as_ptr(), name_len as i32) };
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
}

fn multi_terminal_member_names(symbol: *mut lemon_bindings::symbol) -> Vec<String> {
    unsafe { std::slice::from_raw_parts((*symbol).subsym, (*symbol).nsubsym as usize) }.into_iter()
        .map(|&sub| Symbol::from_raw(0, sub).name())
        .collect()
}

fn precedence_from_raw(sym: *mut lemon_bindings::symbol) -> Option<Precedence> {
    match unsafe { ((*sym).assoc, (*sym).prec) } {
        (lemon_bindings::e_assoc_LEFT, prec) => Some(Precedence::Left(prec)),
        (lemon_bindings::e_assoc_RIGHT, prec) => Some(Precedence::Right(prec)),
        (lemon_bindings::e_assoc_NONE, _) => Some(Precedence::Noassoc),
        (lemon_bindings::e_assoc_UNK, _) => None,
        (assoc, prec) => panic!("Unexpected precedence value (assoc: {assoc}, prec: {prec})"),
    }
}

#[derive(serde::Serialize)]
pub struct Rule {
    lhs: String,
    members: Vec<RuleMember>,
}

impl Rule {
    pub fn from_raw(members: &[*mut lemon_bindings::rule]) -> Self {
        let lhs = Rule::name_from(members[0]);
        let mut members = members.into_iter().map(|&x| RuleMember::from_raw(x)).collect::<Vec<_>>();
        members.sort_by(|m1, m2| m1.index.cmp(&m2.index));

        Self { lhs, members }
    }

    pub fn name_from(rule: *mut lemon_bindings::rule) -> String {
        unsafe { CStr::from_ptr((*(*rule).lhs).name) }
        .to_string_lossy()
        .to_string()
    }
}

pub struct RuleMember {
    index: usize,
    inner: *mut lemon_bindings::rule,
}

impl RuleMember {
    pub fn from_raw(rule: *mut lemon_bindings::rule) -> Self {
        Self { 
            index: unsafe {(*rule).index} as usize,
            inner: rule 
        }
    }

    pub fn precedence(&self) -> Option<Precedence> {
        unsafe {
            match (*self.inner).precsym.is_null() {
                false => {
                    precedence_from_raw((*self.inner).precsym)
                }
                true => None
            }
        }
    }

    pub fn rhs(&self) -> Vec<Rhs> {
        let rhs = unsafe { std::slice::from_raw_parts((*self.inner).rhs, (*self.inner).nrhs as usize) };

        let rhs = rhs.into_iter()
            .map(|&x| Rhs::from_raw(x))
            .collect::<Vec<_>>()
        ;

        rhs
    }
}

pub struct Rhs {
    inner: *mut lemon_bindings::symbol,
}

impl Rhs {
    fn from_raw(inner: *mut lemon_bindings::symbol) -> Self {
        Self { inner }
    }
}

impl serde::Serialize for Rhs {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer 
    {
        let name = unsafe { CStr::from_ptr((*self.inner).name) }
            .to_string_lossy()
            .to_string()
        ;

        let rhs = unsafe {
            match ((*self.inner).type_ == 2, (*self.inner).useCnt == 0) {
                (true, true) => {
                    Term::CharClass { members: multi_terminal_member_names(self.inner) }
                }
                _ => {
                    Term::Symbol { name }
                }
            }
        };
        rhs.serialize(serializer)
    }
}

impl serde::Serialize for RuleMember {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer 
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("rule", 10)?;
        state.serialize_field("id", &(self.index + 1))?;
        state.serialize_field("sequences", &self.rhs())?;
        state.serialize_field("precedence", &self.precedence())?;
        state.end()
    }
}

