use scanner_core::{iter::{StatementScanner, StatementScannerType}, ScannerAccess};
use crate::{incremental::support, syntax_tree::{MetadataAccess, NodeOperation, SyntaxElement, SyntaxNode, SyntaxTree}};

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
        use crate::syntax_tree::MetadataAccess;

        // Skip out of range statements
        let mut iter = old_tree.root().children()
            .filter_map(|node| match node {
                crate::syntax_tree::SyntaxElementDef::Node(node) => {
                    Some((node.clone(), node.metadata().char_range()))
                }
                crate::syntax_tree::SyntaxElementDef::TokenSet(_) => None
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
            EvalState::ForwardScan{ head_anchor: index, tail_anchor: tail_index } => {
                let scanner_start = preceding_len - index;
                let scanner_end = scanners.len() - (following_len - tail_index.unwrap_or_default());
                
                let statements = eval_hint_internal(
                    self.precedings[0..=index].iter().flatten().rev()
                        .chain(self.statements.iter())
                        .chain(self.followings.iter().flatten().take(tail_index.unwrap_or(following_len)))
                        .skip(1),
                    scanners.into_iter().skip(scanner_start).take(scanner_end - scanner_start).rev()
                );

                (scanner_start, statements)
            }
            EvalState::ReverseScan{ head_anchor: index, tail_anchor: tail_index, need_skip } if need_skip => {
                let scanner_start = tail_index.map(|i| preceding_len - i).unwrap_or_default();
                let scanner_end = scanners.len() - (following_len - index);
                
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
}

#[derive(PartialEq)]
enum EvalState {
    ForwardScan{ head_anchor: usize, tail_anchor: Option<usize> },
    ReverseScan{ head_anchor: usize, tail_anchor: Option<usize>, need_skip: bool },
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
        (Some((head_anchor, _)), Some((tail_anchor, need_skip))) if need_skip => {
            EvalState::ForwardScan { head_anchor, tail_anchor: Some(tail_anchor) }
        }
        (Some((head_anchor, _)), Some((tail_anchor, _))) => {
            EvalState::ReverseScan{ head_anchor: tail_anchor, tail_anchor: Some(head_anchor), need_skip: false }
        }
        (None, Some((head_anchor, need_skip))) => {
            EvalState::ReverseScan{ head_anchor: head_anchor, tail_anchor: None, need_skip }
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

fn find_following_anchor_index(followings: &[Option<SyntaxNode>], scanners: &[StatementScanner]) -> Option<(usize, bool)> {
    let siblings = followings.iter().flatten().collect::<Vec<_>>();

    if (siblings.len() == FOLLOWING_SIZE) && scanners.len() >= FOLLOWING_SIZE {
        for i in 0..2 {
            let followings_window = &siblings[i..];
            let scanners_window = &scanners[..];

            if let Some(index) = match_full(followings_window, scanners_window, i) {
                return Some((index, true));
            }
        }
    }

    let followings_window_end = usize::min(siblings.len(), FOLLOWING_SIZE-1);
    let followings_window = &siblings[0..followings_window_end];
    let scanners_window = &scanners[..];

    if let Some(index) = match_tail(followings_window, scanners_window) {
        return Some((index, true));
    }
    
    Some((siblings.len() - 1, false))
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
}