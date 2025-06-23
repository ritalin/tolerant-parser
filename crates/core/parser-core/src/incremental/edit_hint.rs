use engine_core::parser_engine::EmitConfig;
use scanner_core::iter::StatementScanner;

use crate::syntax_tree::{SyntaxElement, SyntaxNode, SyntaxTokenSet, SyntaxTree};


#[derive(PartialEq, Eq, Clone, Debug)]
pub enum EditHint {
    Prepend{ candidate: SyntaxNode },
    Append{ candidate: SyntaxNode },
    Update{ candidates: Vec<SyntaxNode>, replace_from: usize },
    InsertBetween{ left: SyntaxNode, right: SyntaxNode }
}

impl EditHint {
    pub fn replace_from(&self) -> usize {
        match self {
            EditHint::Prepend{ candidate: node } => node.into_raw().index(),
            EditHint::Append{ candidate: node } => node.into_raw().index(),
            EditHint::Update{ replace_from, .. } => *replace_from,
            EditHint::InsertBetween { left: prev, .. } => prev.into_raw().index(),
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
                    (Some((prev, _)), Some((next, _))) => Self::InsertBetween { left: prev.clone(), right: next.clone() },
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
        let mut statements = vec![];
        let mut skip_scanner = 0;
        let mut replace_from = self.replace_from(); 

        let scanner_iter = scanners.iter().take_while(|scanner| scanner.scan_range().start < new_edit_byte_range.end);

        match (self, scanners.first()) {
            (EditHint::Prepend { candidate }, Some(_)) => {
                for scanner in scanner_iter {
                    // If it contains the emit symbol, it is inserting statement.
                    match eval_prepend_hint(scanner, candidate, &new_edit_byte_range, emit_region) {
                        Some(Some(stmt)) => statements.push(Some(stmt)),
                        Some(None) => statements.push(None),
                        None => {}
                    }
                }
            }
            (EditHint::Append{ candidate }, Some(_)) => {
                let end_token_set = super::support::find_last_token_set(candidate);

                for scanner in scanner_iter {
                    match eval_append_hint(scanner, candidate, end_token_set.as_ref(), &new_edit_byte_range, emit_region) {
                        Some(Some(stmt)) => {
                            statements.push(Some(stmt));
                        }
                        Some(None) if statements.is_empty() => {
                            statements.push(None);
                            skip_scanner += 1;
                            replace_from += 1;
                        }
                        Some(None) => {
                            statements.push(None)
                        }
                        None => {
                            // out of range
                        }
                    }
                }
            }
            (EditHint::InsertBetween { left, right }, Some(_)) => {
                #[derive(PartialEq)]
                enum InsertEvalState {
                    AppendEnabled,
                    Appending,
                    PrependEnabled,
                    Prepending,
                }

                let left_end_token_set = super::support::find_last_token_set(left);
                let mut state = InsertEvalState::AppendEnabled;

                for scanner in scanner_iter {
                    match state {
                        InsertEvalState::AppendEnabled | InsertEvalState::Appending => {
                            // eval as append
                            match eval_append_hint(scanner, left, left_end_token_set.as_ref(), &new_edit_byte_range, emit_region) {
                                Some(Some(stmt)) => {
                                    // update statement
                                    statements.push(Some(stmt));
                                    state = InsertEvalState::Appending;
                                }
                                Some(None) if state == InsertEvalState::Appending => {
                                    // append new statement
                                    statements.push(None);
                                }
                                _ => {
                                    // disable to append
                                    state = InsertEvalState::PrependEnabled;
                                }
                            };
                        }
                        InsertEvalState::PrependEnabled | InsertEvalState::Prepending => {
                            // eval as prepend
                            match eval_prepend_hint(scanner, right, &new_edit_byte_range, emit_region) {
                                Some(Some(stmt)) => {
                                    // update statement
                                    statements.push(Some(stmt));
                                    state = InsertEvalState::Prepending;
                                }
                                Some(None) => {
                                    // prepend new statement
                                    statements.push(None);
                                    state = InsertEvalState::Prepending;
                                }
                                None => {}
                            }
                        }
                    }
                }

                // if accept prepend, shift scanner
                if state == InsertEvalState::Prepending { 
                    skip_scanner += 1;
                    replace_from += 1;
                };
            }
            (EditHint::Update { candidates, .. }, _) => {
                // FIXME: unmatched old statement count and scanners count
                let scanner_count = scanner_iter.count();
                statements.extend(candidates.iter().take(scanner_count).cloned().map(Some).collect::<Vec<_>>());
                
                if statements.len() < scanner_count { 
                    // the statement is splitted
                    statements.extend(vec![None; scanner_count - statements.len()]) 
                }
            }
            (_, None)  => {
                // No change         
            }
        }

        EditHintEval{ statements, skip_scanner, replace_from }
    }
}

fn eval_append_hint(scanner: &StatementScanner, stmt: &SyntaxNode, end_token_set: Option<&SyntaxTokenSet>, edit_scope_range: &std::ops::Range<usize>, emit_region: &EmitConfig) -> Option<Option<SyntaxNode>> {
    use scanner_core::ScannerAccess;
    use crate::syntax_tree::MetadataAccess;

    let scan_from = usize::max(scanner.scan_range().start, edit_scope_range.start);
    let scan_to = usize::min(scanner.scan_range().end, edit_scope_range.end);

    match (scanner.as_view(scan_from..scan_to).lookahead(), end_token_set.as_ref()) {
        (Some(lookahead), Some(token_set)) if token_set.metadata_key().offset == lookahead.main.offset => {
            // update prev statement by appending trailing trivia
            Some(Some(stmt.clone()))
        }
        (_, Some(token_set)) if token_set.metadata_key().kind != emit_region.to_symbol => {
            // update statement but not append
            Some(Some(stmt.clone()))
        }
        (Some(_), _) => {
            Some(None)
        }
        (None, _) => {
            // out of range
            None
        }
    }
}

fn eval_prepend_hint(scanner: &StatementScanner, stmt: &SyntaxNode, edit_scope_range: &std::ops::Range<usize>, emit_region: &EmitConfig) -> Option<Option<SyntaxNode>> {
    use scanner_core::ScannerAccess;

    let scan_from = usize::min(scanner.scan_range().end, edit_scope_range.end);
    
    match scanner.as_view((scan_from.saturating_sub(1))..).lookahead() {
        Some(lookahead) if lookahead.main.kind == emit_region.to_symbol => {
            // prepend statement
            Some(None)
        }
        _ => {
            // update statement 
            Some(Some(stmt.clone()))
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct EditHintEval {
    pub statements: Vec<Option<SyntaxNode>>,
    pub skip_scanner: usize,
    pub replace_from: usize,
}

