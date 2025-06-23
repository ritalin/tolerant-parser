use engine_core::parser_engine::EmitConfig;
use scanner_core::iter::StatementScanner;

use crate::syntax_tree::{SyntaxElement, SyntaxNode, SyntaxTree};


#[derive(PartialEq, Eq, Clone, Debug)]
pub enum EditHint {
    Prepend{ candidate: SyntaxNode },
    Append{ candidate: SyntaxNode },
    Update{ candidates: Vec<SyntaxNode>, replace_from: usize },
    InsertBetween{ prev: SyntaxNode, next: SyntaxNode }
}

impl EditHint {
    pub fn replace_from(&self) -> usize {
        match self {
            EditHint::Prepend{ candidate: node } => node.into_raw().index(),
            EditHint::Append{ candidate: node } => node.into_raw().index(),
            EditHint::Update{ replace_from, .. } => *replace_from,
            EditHint::InsertBetween { prev, .. } => prev.into_raw().index(),
        }
    }

    pub fn new(old_tree: &SyntaxTree, range: std::ops::Range<usize>, eof_statement: Option<&SyntaxElement>) -> Self {
        use crate::syntax_tree::MetadataAccess;

        let eof_statement = eof_statement.and_then(|el| el.to_node());

        // Skip out of range statements
        let mut iter = old_tree.root().children()
            .filter_map(|node| match node {
                crate::syntax_tree::SyntaxElementDef::Node(node) => {
                    Some((node.clone(), node.metadata()))
                }
                crate::syntax_tree::SyntaxElementDef::TokenSet(_) => None
            })
            .skip_while(move |(_, metadata)| metadata.char_range().end < range.start)
            .peekable()
        ;

        match iter.peek() {
            None => {
                Self::Update{ candidates: vec![], replace_from: 0 }
            }
            Some((node, _)) if eof_statement.as_ref().zip(Some(node)).filter(|(node, needle)| node == needle).is_some() => {
                //  If EOF statement
                Self::Update{ candidates: vec![], replace_from: node.into_raw().index() }
            }
            Some((_, metadata)) if (metadata.char_offset + metadata.char_len == range.start) && (range.start == range.end) => {
                match (iter.next(), iter.next()) {
                    (Some((prev, _)), Some((next, _))) if eof_statement.as_ref().zip(Some(&next)).filter(|(node, needle)| node == needle).is_some() => {
                        Self::Append{ candidate: prev.clone() }
                    }
                    (Some((prev, _)), Some((next, _))) => Self::InsertBetween { prev: prev.clone(), next: next.clone() },
                    (Some((prev, _)), None) => Self::Update{ candidates: vec![], replace_from: prev.into_raw().index() }, // same as EOF statement
                    _ => unreachable!("Peek returned Some, but next was None")
                }
            }
            Some((first_node, _)) if (range.start == range.end) && (range.start == 0) => {
                Self::Prepend { candidate: first_node.clone() }
            }
            Some((first_node, _)) => {
                let replace_from = first_node.into_raw().index();
                let nodes = iter
                    .take_while(|(_, metadata)| {
                        metadata.char_offset < range.end
                    })
                    .map(|(node, _)| node.clone())
                    .collect::<Vec<_>>()
                ;
                Self::Update { candidates: nodes, replace_from }
            }
        } 
    }

    pub fn eval_hint(&self, scanners: &Vec<StatementScanner>, new_edit_byte_range: std::ops::Range<usize>, emit_region: &EmitConfig) -> EditHintEval {
        use scanner_core::ScannerAccess;

        let mut skip_scanner = 0;
        let replace_from = self.replace_from(); 

        match (self, scanners.first()) {
            (EditHint::Prepend { candidate }, Some(scanner)) => {
                // FIXME: contains many statements but case of non terminated semicollon 
                // If it contains the emit symbol, it is inserting statement.
                match scanner.as_view((new_edit_byte_range.end.saturating_sub(1))..).lookahead() {
                    Some(lookahead) if lookahead.main.kind == emit_region.to_symbol => {
                        // prepend statement
                        EditHintEval{ statements: vec![], skip_scanner: 0, replace_from }
                    }
                    _ => {
                        // uodate statement 
                        EditHintEval{ statements: vec![Some(candidate.clone())], skip_scanner: 0, replace_from }
                    }
                }
            }
            (EditHint::Append{ candidate }, Some(scanner)) => {
                match eval_append_hint(scanner.as_view(new_edit_byte_range.start..), candidate, emit_region, 0, replace_from) {
                    Some(result) => result,
                    None => {
                        // append new statement
                        EditHintEval{ statements: vec![], skip_scanner: 1, replace_from: replace_from + 1 }
                    }
                }
            }
            (EditHint::InsertBetween { prev, next }, Some(scanner)) => {
                // eval as append
                if let Some(result) = eval_append_hint(scanner.as_view(new_edit_byte_range.start..), prev, emit_region, skip_scanner, replace_from) {
                    return result;
                }

                skip_scanner += 1;

                if let Some(scanner) = scanners.get(skip_scanner) {
                    // eval as prepend
                    match scanner.as_view((new_edit_byte_range.end.saturating_sub(1))..).lookahead() {
                        Some(lookahead) if lookahead.main.kind != emit_region.to_symbol => {
                            // update next statement
                            return EditHintEval{ statements: vec![Some(next.clone())], skip_scanner: 1, replace_from: replace_from + 1 };
                        }
                        _ => {}
                    }                        
                }

                // insert statements
                EditHintEval{ statements: vec![], skip_scanner: 1, replace_from: replace_from + 1 }
            }
            (EditHint::Update { candidates, .. }, _) => {
                // FIXME: unmatched old statement count and scanners count
                EditHintEval{ statements: (candidates.iter().cloned().map(Some).collect()), skip_scanner: 0, replace_from }
            }
            (_, None)  => {
                // No change
                EditHintEval{ statements: vec![], skip_scanner: 0, replace_from }
            }
        }
    }
}

fn eval_append_hint(scanner: impl scanner_core::ScannerAccess, stmt: &SyntaxNode, emit_region: &EmitConfig, skip_scanner: usize, replace_from: usize) -> Option<EditHintEval> {
    use crate::syntax_tree::MetadataAccess;

    let end_token_set = super::support::find_last_token_set(stmt);

    match (scanner.lookahead(), end_token_set) {
        (Some(lookahead), Some(token_set)) if token_set.metadata_key().offset == lookahead.main.offset => {
            // update prev statement by appending trailing trivia
            Some(EditHintEval{ statements: vec![Some(stmt.clone())], skip_scanner, replace_from })
        }
        (_, Some(token_set)) if token_set.metadata_key().kind != emit_region.to_symbol => {
            // update statement but not append
            Some(EditHintEval{ statements: vec![Some(stmt.clone())], skip_scanner, replace_from })
        }
        _ => {
            None
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct EditHintEval {
    pub statements: Vec<Option<SyntaxNode>>,
    pub skip_scanner: usize,
    pub replace_from: usize,
}

