use std::{ffi::{CStr, CString}, mem::MaybeUninit};
use crate::keyword_check::sqlite3_keyword_check;

use super::lemon_bindings;

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
        
            // let sym_eof = lemon_bindings::Symbol_new(CString::new("EOF").unwrap().as_ptr());

            // let start_rule = rule_raw_by_name(&mut self.inner, "input");
            // expand_rhs_symbol(start_rule, &[sym_eof]);

            self.inner.nsymbol = lemon_bindings::Symbol_count();
            self.inner.symbols = lemon_bindings::Symbol_arrayof();
        };

    }

    pub fn symbols(&self) -> Vec<Symbol> {
        self.symbols_internal().collect()
    }

    pub fn symbols_internal(&self) -> impl Iterator<Item = Symbol> {
        unsafe { std::slice::from_raw_parts(self.inner.symbols, self.inner.nsymbol as usize) }.into_iter()
        .enumerate()
        .map(|(i, &x)| Symbol::from_raw(i + 1, x))
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
        Precedence::from_raw(self.inner)
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

#[derive(PartialEq, Eq, Ord, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Precedence {
    Left(i32),
    Right(i32),
    Noassoc,
}

impl Precedence {
    pub fn from_raw(sym: *mut lemon_bindings::symbol) -> Option<Precedence> {
        match unsafe { ((*sym).assoc, (*sym).prec) } {
            (lemon_bindings::e_assoc_LEFT, prec) => Some(Precedence::Left(prec)),
            (lemon_bindings::e_assoc_RIGHT, prec) => Some(Precedence::Right(prec)),
            (lemon_bindings::e_assoc_NONE, _) => Some(Precedence::Noassoc),
            (lemon_bindings::e_assoc_UNK, _) => None,
            (assoc, prec) => panic!("Unexpected precedence value (assoc: {assoc}, prec: {prec})"),
        }
    }

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
