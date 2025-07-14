use std::collections::VecDeque;

use crate::core::{engine_core::scanner_engine::{CaseSensitivity, ScanningRuleSet}, scanner_core::{iter::{StatementScanner, StatementScannerType}, Scanner, ScannerAccess, ScannerConfig}};
use crate::core::parser_core::{self, incremental::support, syntax_tree::{MetadataAccess, NodeOperation, SyntaxElement, SyntaxNode, SyntaxTree}};
use super::extract_lookahead;

/// Contains information about the edit region and its surrounding statements.
///
/// This struct is used for incremental re-parsing. It divides the syntax tree into:
/// - `statements`: the statements that intersect with the edited range.
/// - `precedings`: up to two statements before the edited range (ordered outer to inner).
/// - `followings`: up to two statements after the edited range (ordered inner to outer).
///
/// Examples:
/// - If the edit covers `statement#1`, and the full list is `[stmt#0, statement#1, stmt#2, EOF]`:
///     - `statements = [statement#1]`
///     - `precedings = [Some(stmt#0)]`
///     - `followings = [Some(stmt#2)]`
/// - If the edit is empty and at the end:
///     - `statements = []`
///     - `followings = [EOF]`
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct EditHint {
    /// Statements fully or partially covered by the edited range.
    pub statements: Vec<SyntaxNode>,
    /// Up to two statements preceding the edited range.
    pub precedings: [Option<SyntaxNode>; 2], 
    /// Up to two statements following the edited range.
    pub followings: [Option<SyntaxNode>; 3],
}

const FOLLOWING_SIZE: usize = 3;

impl EditHint {
    pub fn scan_from(&self) -> usize {
        let stmt = self.precedings.iter().flatten().last()
            .or(self.statements.first())
            .or(self.followings.first().and_then(|node| node.as_ref()))
            .expect("At least, `followings.first()` hits at EOF statement")
        ;
        stmt.metadata_key().offset
    }

    pub fn new(old_tree: &SyntaxTree, range: std::ops::Range<usize>) -> Self {
        use parser_core::syntax_tree::MetadataAccess;

        // Skip out of range statements
        let mut iter = old_tree.root().children()
            .filter_map(|node| match node {
                parser_core::syntax_tree::SyntaxElementDef::Node(node) => {
                    Some((node.clone(), node.metadata().char_range()))
                }
                parser_core::syntax_tree::SyntaxElementDef::TokenSet(_) => None
            })
            .skip_while(move |(_, char_range)| {
                char_range.end < range.start
            })
            .peekable()
        ;

        // pick up precedings
        let left = 'precedings: {
            if let Some((stmt, char_range)) = iter.peek() {
                break 'precedings match char_range.end <= range.start {
                    true if stmt.next_sibling().is_some() => {
                        let node = stmt.clone();
                        iter.next();
                        Some(SyntaxElement::Node(node))
                    }
                    false if char_range.len() > 0 => {
                        stmt.prev_sibling()
                    }
                    _ => None
                }
            }
            None
        };

        let precedings = std::iter::successors(left, |node| node.prev_sibling())
            .take(2)
            .enumerate()
            .fold([None, None], |mut acc, (i, node)| {
                acc[i] = node.to_node();
                acc
            })
        ;
        
        // pick up within range
        let mut statements = vec![];
        while let Some((stmt, char_range)) = iter.peek() {
            match ((char_range.start < range.end), (char_range.end > range.start)) {
                (true, true) if stmt.next_sibling().is_some() => {
                    statements.push(stmt.clone());
                    iter.next();
                }
                _ => break,
            }
        }
                
        // pick up followings
        let right = iter.next().map(|(node, _)| SyntaxElement::Node(node.clone()));
        let followings = std::iter::successors(right, |node| node.next_sibling())
            .take(FOLLOWING_SIZE)
            .enumerate()
            .fold([const { None }; FOLLOWING_SIZE], |mut acc, (i, node)| {
                acc[i] = node.to_node();
                acc
            })
        ;
        
        Self { statements, precedings, followings }
    }

    pub fn eval_hint(&self, scanners: Vec<StatementScanner>, new_edit_byte_range: std::ops::Range<usize>) -> EditHintSlots {
        let scanners = extend_statement_scanners(scanners, &new_edit_byte_range);
        let preceding_len = self.precedings.iter().flatten().count();
        let following_len = self.followings.iter().flatten().count();

        let (skip_scanner, events) = match find_anchor_index(&self.precedings, &self.followings, &scanners) {
            EvalState::ForwardScan{ head_anchor: index, tail_anchor: tail_index, tail_window_size } => {
                let tail_window_size = if self.statements.is_empty() && (following_len != tail_window_size) { following_len } else { tail_window_size };
                let scanner_start = preceding_len - index;
                // let scanner_end = scanners.len() - (following_len - tail_index.unwrap_or_default());
                let scanner_end = scanners.len() - tail_index.map(|i| tail_window_size - i).unwrap_or_default();
                
                let statements = eval_hint_internal(
                    self.precedings[0..=index].iter().flatten().rev()
                        .chain(self.statements.iter())
                        .chain(self.followings.iter().flatten().take(tail_index.unwrap_or(following_len)))
                        .skip(1),
                    scanners.into_iter().skip(scanner_start).take(scanner_end - scanner_start)
                );

                (scanner_start, statements)
            }
            EvalState::ReverseScan{ head_anchor: index, tail_anchor: tail_index, need_skip , tail_window_size: window_size} if need_skip => {
                let scanner_start = tail_index.map(|i| preceding_len - i).unwrap_or_default();
                let scanner_end = scanners.len() - (window_size - index);
                
                let mut statements = eval_hint_internal(
                    self.precedings.iter().flatten().rev()
                        .chain(self.statements.iter())
                        .chain(self.followings[0..=index].iter().flatten())
                        .rev()
                        .skip(1),
                    scanners.into_iter().skip(scanner_start).take(scanner_end - scanner_start).rev()
                );
                statements.reverse();
                (scanner_start, statements)
            }
            EvalState::ReverseScan{ head_anchor: index, tail_anchor: tail_index, .. } => {
                let scanner_start = tail_index.map(|i| preceding_len - i).unwrap_or_default();
                let scanner_end = scanners.len();
                
                let mut statements = eval_hint_internal(
                    self.precedings[0..(tail_index.unwrap_or(preceding_len))].iter().flatten().rev()
                        .chain(self.statements.iter())
                        .chain(self.followings[0..=index].iter().flatten())
                        .rev(),
                    scanners.into_iter().skip(scanner_start).take(scanner_end - scanner_start).rev()
                );
                statements.reverse();
                (scanner_start, statements)
            }
        };

        let replace_from_range = extract_replace_range(&events);

        EditHintSlots{ 
            events, 
            replace_from: replace_from(self, skip_scanner), 
            replace_byte_range: replace_from_range,
        }
    }
    
    pub fn reconcile_lookaheads(&self, old_char_range: std::ops::Range<usize>, text: &str, engine: ScanningRuleSet, case_sensitive: CaseSensitivity) -> Result<VecDeque<crate::core::scanner_core::Token>, super::ParseError> {
        let mut lookaheads = VecDeque::with_capacity(32);

        let precedings = self.precedings.iter().flatten().rev().cloned().collect::<Vec<_>>();
        
        'head_clean_lookaheads: {
            // Resolve head clean lookaheads 
            let sentinel = if self.statements.is_empty() { support::find_first_token_set(precedings.last()) } else { None };
            let token_sets = extract_lookahead::pick_clean_head_token_sets(&precedings, sentinel);

            extract_lookahead::extract_clean_lookaheads(token_sets, None, &engine, &mut lookaheads);

            break 'head_clean_lookaheads; 
        };
        'dirty_lookaheads: {
            let start_replace_stmt = self.statements.first().or_else(|| precedings.last());
            
            // resolve head clean part
            let mut head_dirty_tokenset = None;
            let mut head_token_sets = VecDeque::new();
            let start_token_set = support::find_first_token_set(start_replace_stmt);
            extract_lookahead::extract_head_clean_lookaheads_forwards(start_token_set, old_char_range.start, &mut head_token_sets, &mut head_dirty_tokenset);
            
            // resolve tail clean part
            let end_token_set = support::find_last_token_set(self.statements.last());
            let mut tail_dirty_tokenset = None;
            let mut tail_token_sets = VecDeque::new();
            extract_lookahead::extract_tail_clean_lookaheads_backwards(end_token_set, old_char_range.end, &mut tail_token_sets, &mut tail_dirty_tokenset);

            // resolve dirty part
            let (start_offset, buf) = extract_lookahead::concat_dirty_text(head_dirty_tokenset.as_ref(), text, tail_dirty_tokenset.as_ref(), old_char_range);
            let mut scanner = Scanner::create_without_scan(&buf, 0, engine.clone(), ScannerConfig{ case_sensitive, offset_with: start_offset})?;
            let full_emit_kind = engine.eof();
            let dirty_lookaheads = scanner.prefetch_iter(full_emit_kind)
                .filter(|x| x.main.kind != full_emit_kind)
                .cloned()
            ;

            extract_lookahead::extract_clean_lookaheads(head_token_sets, None, &engine, &mut lookaheads);
            lookaheads.extend(dirty_lookaheads);
            extract_lookahead::extract_clean_lookaheads(tail_token_sets, Some(start_offset + buf.len()), &engine, &mut lookaheads);

            break 'dirty_lookaheads;
        };
        'tail_clean_lookaheads: {
            // Resolve tail clean lookaheads
            let followings = self.followings.iter().flatten().cloned().collect::<Vec<_>>();
            let token_sets = extract_lookahead::pick_clean_tail_token_sets(&followings);

            let start_tail_offset = lookaheads.back().map(|x| x.token_range().end).unwrap_or_default();
            extract_lookahead::extract_clean_lookaheads(token_sets, Some(start_tail_offset), &engine, &mut lookaheads);

            break 'tail_clean_lookaheads;
        };

        Ok(lookaheads)
    }
}

#[derive(PartialEq)]
enum EvalState {
    ForwardScan{ head_anchor: usize, tail_anchor: Option<usize>, tail_window_size: usize },
    ReverseScan{ head_anchor: usize, tail_anchor: Option<usize>, tail_window_size: usize, need_skip: bool },
}

fn extend_statement_scanners(scanners: Vec<StatementScanner>, byte_range: &std::ops::Range<usize>) -> Vec<StatementScanner> {
    let mut result = Vec::with_capacity(scanners.len());
    let mut iter = scanners.into_iter();
    let mut left = FOLLOWING_SIZE;

    while let Some(scanner) = iter.next() {        
        if scanner.scan_range().end > byte_range.end {
            if left == 0 { break }
            left -= 1;
        }
        result.push(scanner);
    }

    result
}

fn find_anchor_index(precedings: &[Option<SyntaxNode>], followings: &[Option<SyntaxNode>], scanners: &[StatementScanner]) -> EvalState {
    let preceding_anchor = find_preceding_anchor_index(precedings, scanners);
    let following_anchor = find_following_anchor_index(followings, scanners);

    match (preceding_anchor, following_anchor) {
        (Some((head_anchor, _)), Some((tail_anchor, need_skip, window_size))) if need_skip => {
            EvalState::ForwardScan { head_anchor, tail_anchor: Some(tail_anchor), tail_window_size: window_size }
        }
        (Some((head_anchor, _)), Some((tail_anchor, _, window_size))) => {
            EvalState::ReverseScan{ head_anchor: tail_anchor, tail_anchor: Some(head_anchor), need_skip: false, tail_window_size: window_size }
        }
        (None, Some((head_anchor, need_skip, window_size))) => {
            EvalState::ReverseScan{ head_anchor: head_anchor, tail_anchor: None, need_skip, tail_window_size: window_size }
        }
        (Some(_), None) => {
            unreachable!("followings must contain at least EOF and match as anchor")
        }
        (None, None) => {
            unreachable!("followings must contain at least EOF and match as anchor")
        }
    }
}

fn find_preceding_anchor_index(siblings: &[Option<SyntaxNode>], scanners: &[StatementScanner]) -> Option<(usize, bool)> {
    let end = siblings.iter().flatten().count();
    let scanners = &scanners[0..end];

    for (i, sibling) in siblings.iter().enumerate() {
        let Some(stmt) = sibling else { continue };

        for scanner in scanners.iter().rev() {
            if (scanner.scanner_type() == StatementScannerType::Statement) && (stmt.metadata_key().byte_range() == scanner.scan_range()) {
                return Some((i, true));
            }
        }
    }
    
    None
}

fn find_following_anchor_index(followings: &[Option<SyntaxNode>], scanners: &[StatementScanner]) -> Option<(usize, bool, usize)> {
    let siblings = followings.iter().flatten().collect::<Vec<_>>();

    if (siblings.len() == FOLLOWING_SIZE) && scanners.len() >= FOLLOWING_SIZE {
        for i in 0..2 {
            let followings_window = &siblings[i..];
            let scanners_window = &scanners[..];

            if let Some(index) = match_full(followings_window, scanners_window, i) {
                return Some((index, true, followings_window.len()));
            }
        }
    }

    let followings_window_end = usize::min(siblings.len(), FOLLOWING_SIZE-1);
    let followings_window = &siblings[0..followings_window_end];
    let scanners_window = &scanners[..];

    if let Some(index) = match_tail(followings_window, scanners_window) {
        return Some((index, true, followings_window_end));
    }
    
    Some((siblings.len() - 1, false, siblings.len()))
}

fn match_full(followings: &[&SyntaxNode], scanners: &[StatementScanner], slide: usize) -> Option<usize> {
    let mut scanner_iter = scanners.iter().rev();
    
    for following in followings.iter().rev() {
        let Some(scanner) = scanner_iter.next() else { break };
        if ! match_statement(following, scanner) { 
            return None;
        }
    }
    
    Some(slide)
}

fn match_tail(followings: &[&SyntaxNode], scanners: &[StatementScanner]) -> Option<usize> {
    let mut best_match = None;

    let mut scanner_iter = scanners.iter().rev();
    
    for (i, following) in followings.iter().enumerate().rev() {
        let Some(scanner) = scanner_iter.next() else { break };
        if ! match_statement(following, scanner) { break }

        best_match = Some(i);
    }

    best_match
}

fn match_statement(stmt: &SyntaxNode, scanner: &StatementScanner) -> bool {
    if scanner.scan_range().len() != stmt.metadata_key().byte_range().len() { return false }
    let Some(first_token_set) = support::find_first_token_set(Some(stmt)) else { return false };

    let scanner_view = scanner.as_view(..);
    let Some(lookahead) = scanner_view.lookahead() else { return false };
    let mut trivia_nodes = first_token_set.leading_trivia().peekable();
    
    match lookahead.leading_trivia.as_ref() {
        Some(trivias) => {
            let matched = trivias.iter().all(|new_token| {
                let node = trivia_nodes.next();
                let (node_kind, node_value) = node.as_ref()
                    .map(|node| (Some(node.metadata_key().kind), Some(node.value())))
                    .unwrap_or((None, None))
                ;

                (node_kind == Some(new_token.kind)) && (node_value == new_token.value.as_deref())
            });
            matched && trivia_nodes.peek().is_none()
        }
        None => trivia_nodes.peek().is_none(),
    }
}

fn eval_hint_internal<'a>(mut statements: impl Iterator<Item = &'a SyntaxNode>, mut scanners: impl Iterator<Item = StatementScanner>) -> Vec<SlotEvent> {
    let mut result = Vec::with_capacity(statements.size_hint().0 + scanners.size_hint().0);

    loop {
        match (statements.next(), scanners.next()) {
            (Some(stmt), Some(scanner)) => {
                result.push(SlotEvent::Replacing { node: stmt.clone(), scanner });
            }
            (Some(stmt), None) => {
                result.push(SlotEvent::Deleting { node: stmt.clone() });
            }
            (None, Some(scanner)) => {
                result.push(SlotEvent::Inserting { scanner });
            }
            (None, None) => {
                break;
            }
        }
    }
    
    result
}

fn replace_from(hint: &EditHint, skip: usize) -> usize {
    let stmt = hint.precedings.iter().flatten().rev()
        .chain(&hint.statements)
        .chain(hint.followings.iter().flatten())
        .skip(skip)
        .next()
        .expect("At least, `followings.first()` hits at EOF statement")
    ;
    stmt.into_raw().index()
}

fn extract_replace_range(slots: &[SlotEvent]) -> Option<std::ops::Range<usize>> {
    let start = slots.iter().find_map(|slot| match slot {
        SlotEvent::Replacing { node, .. } | SlotEvent::Deleting { node } => Some(node.metadata_key().offset),
        SlotEvent::Inserting { .. } => None,
    });
    let end = slots.iter().rev().find_map(|slot| match slot {
        SlotEvent::Replacing { node, .. } | SlotEvent::Deleting { node } => Some(node.metadata_key().byte_range().end),
        SlotEvent::Inserting { .. } => None,
    });

    start.zip(end).map(|(start, end)| start..end)
}

#[derive(PartialEq, Debug)]
pub struct EditHintSlots {
    pub events: Vec<SlotEvent>,
    pub replace_from: usize,
    pub replace_byte_range: Option<std::ops::Range<usize>>,
}

#[derive(PartialEq, Debug)]
pub enum SlotEvent {
    Replacing{ node: SyntaxNode, scanner: StatementScanner },
    Deleting{ node: SyntaxNode },
    Inserting{ scanner: StatementScanner },
}

impl SlotEvent {
    pub fn index(&self) -> Option<usize> {
        match self {
            SlotEvent::Replacing { node, .. } => Some(node.into_raw().index()),
            SlotEvent::Deleting { node } => Some(node.into_raw().index()),
            SlotEvent::Inserting { .. } => None,
        }
    }

    pub fn scanner(&self) -> Option<&StatementScanner> {
        match self {
            SlotEvent::Replacing { scanner, .. } => Some(&scanner),
            SlotEvent::Deleting { .. } => None,
            SlotEvent::Inserting { scanner } => Some(&scanner),
        }
    }
}