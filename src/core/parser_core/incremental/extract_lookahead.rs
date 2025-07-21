use std::{collections::VecDeque, ops::Bound};

use crate::core::{engine_core::scanner_engine::{ScanEvent, ScanningRuleSet}, parser_core::{self, incremental::support, syntax_tree::{MetadataAccess, SyntaxNode, SyntaxTokenItem, SyntaxTokenSet}, PatchAction}, scanner_core::Token};

pub fn extract_head_clean_lookaheads_forwards(start_token_set: Option<SyntaxTokenSet>, needle: usize, clean_token_sets: &mut VecDeque<SyntaxTokenSet>, dirty_tokenset: &mut Option<SyntaxTokenSet>) {
    let mut next_token_set = start_token_set;

    while let Some(token_set) = next_token_set {
        let metadata = token_set.metadata();

        match metadata.char_range() {
            r if r.end < needle =>  {
                next_token_set = support::find_next_token_set(Some(&token_set), None);
                clean_token_sets.push_back(token_set);
            }
            r if (r.start <= needle) && (r.end >= needle) => {
                *dirty_tokenset = Some(token_set);
                break;
            }
            _ => {
                break;
            }
        }
    }
}

pub fn extract_tail_clean_lookaheads_backwards(start_token_set: Option<SyntaxTokenSet>, needle: usize, clean_token_sets: &mut VecDeque<SyntaxTokenSet>, dirty_tokenset: &mut Option<SyntaxTokenSet>) {
    let mut prev_token_set = start_token_set;

    while let Some(token_set) = prev_token_set {
        let metadata = token_set.metadata();

        match metadata.char_range() {
            r if r.start > needle => {
                prev_token_set = support::find_prev_token_set(Some(&token_set), None);
                clean_token_sets.push_front(token_set);
            }
            r if (r.start <= needle) && (r.end >= needle) => {
                *dirty_tokenset = Some(token_set);
                break;
            }
            _ => {
                break;
            }
        }
    }
}

pub fn concat_dirty_text(
    head_token_set: Option<&SyntaxTokenSet>, 
    tail_token_set: Option<&SyntaxTokenSet>,
    tail_next_token_set: Option<&SyntaxTokenSet>,
    text: &str, 
    old_char_range: &std::ops::Range<usize>) -> (usize, String) 
{
    let (start_byte_offset, insert_byte_offset, mut buf) = match (head_token_set.as_ref(), tail_token_set.as_ref()) {
        (Some(lhs), Some(rhs)) if lhs == rhs => {
            let mut buf = String::with_capacity(lhs.metadata_key().len + text.len());
            let (start_offset, insert_offset) = update_dirty_text_internal(lhs.descendant_tokens(), &old_char_range, &mut buf);

            (start_offset, insert_offset, buf)
        }
        (Some(lhs), Some(rhs)) => {
            let mut buf = String::with_capacity(lhs.metadata_key().len + rhs.metadata_key().len + text.len());
            let (start_offset, insert_offset) = update_dirty_text_internal(lhs.descendant_tokens(), &old_char_range, &mut buf);
            update_dirty_text_internal(rhs.descendant_tokens(), &old_char_range, &mut buf);

            (start_offset, insert_offset, buf)
        }
        (Some(lhs), None) => {
            let mut buf = String::with_capacity(lhs.metadata_key().len + text.len());
            let (start_offset, insert_offset) = update_dirty_text_internal(lhs.descendant_tokens(), &old_char_range, &mut buf);

            (start_offset, insert_offset, buf)
        }
        (None, Some(rhs)) => {
            let mut buf = String::with_capacity(rhs.metadata_key().len + text.len());
            let (start_offset, insert_offset) = update_dirty_text_internal(rhs.descendant_tokens(), &old_char_range, &mut buf);

            (start_offset, insert_offset, buf)
        }
        (None, None) => {
            return (0, text.to_string());
        }
    };

    if let Some(token_set) = tail_next_token_set {
        update_dirty_text_internal(token_set.descendant_tokens(), &old_char_range, &mut buf);
    }

    buf.insert_str(insert_byte_offset - start_byte_offset, text);

    (start_byte_offset, buf)
}

fn update_dirty_text_internal(token_items: impl Iterator<Item = SyntaxTokenItem>, edit_char_range: &std::ops::Range<usize>, buf: &mut String) -> (usize, usize) {
    let mut iter = token_items
        .map(|item| (item.metadata_key().byte_range(), item.metadata().char_range(), item))
        .filter_map(|(byte_range, char_offset, item)| match char_offset {
            r if (r.start >= edit_char_range.start) && (r.end <= edit_char_range.end) => {
                Some((byte_range.start, (Bound::Unbounded, Bound::Unbounded), None))
            }
            r if (r.end <= edit_char_range.start) => {
                Some((byte_range.start, (Bound::Included(byte_range.end), Bound::Unbounded),  Some(item.value().to_string())))
            } 
            r if (r.start >= edit_char_range.end) => {
                Some((byte_range.start, (Bound::Unbounded, Bound::Included(byte_range.start)), Some(item.value().to_string())))
            }
            r if (edit_char_range.start >= r.start) && (edit_char_range.end <= r.end) => {
                let value = item.value();
                let mut new_value = String::with_capacity(value.len());
                let s1 = substring_as_utf16_range(value, ..(edit_char_range.start - r.start));
                let s2 = substring_as_utf16_range(value, (edit_char_range.end - r.start)..);
                new_value.push_str(&s1);
                new_value.push_str(&s2);

                Some((byte_range.start, (Bound::Included(byte_range.start + s1.len()), Bound::Included(byte_range.start + s1.len())), Some(new_value)))
            }
            r if r.start <= edit_char_range.start => {
                let value = item.value();
                let new_value = substring_as_utf16_range(value, ..(edit_char_range.start - r.start));
                Some((byte_range.start, (Bound::Included(byte_range.start + value.len()), Bound::Unbounded), Some(new_value)))
            }
            r if r.end >= edit_char_range.end => {
                let value = item.value();
                let new_value = substring_as_utf16_range(value, (edit_char_range.end - r.start)..);
                Some((byte_range.start, (Bound::Unbounded, Bound::Included(byte_range.start)), Some(new_value)))
            }
            _ => None
        })
        .peekable()
    ;

    let start_offset = iter.peek().map(|(start, _, _)| *start).unwrap_or_default();
    let mut insert_start_best = None;
    let mut insert_end_best = None;

    for (_, (insert_start, insert_end), value) in iter {
        if let Some(v) = value {
            buf.push_str(&v);
        }

        match (insert_start, insert_start_best) {
            (Bound::Included(i), None) => insert_start_best = Some(i),
            (Bound::Included(i), Some(candidate)) => insert_start_best = Some(usize::max(i, candidate)),
            (Bound::Unbounded, None) => insert_start_best = Some(start_offset),
            _ => {}
        };
        match (insert_end, insert_end_best) {
            (Bound::Included(i), None) => insert_end_best = Some(i),
            (Bound::Included(i), Some(candidate)) => insert_end_best = Some(usize::min(i, candidate)),
            _ => {}
        }
    }

    let insert_offset = match (insert_start_best, insert_end_best) {
        (None, None) => start_offset,
        (None, Some(offset)) | (Some(offset), None) => offset,
        (Some(lhs), Some(rhs)) => usize::min(lhs, rhs),
    };

    (start_offset, insert_offset)
}

fn substring_as_utf16_range(value: &str, char_range: impl std::ops::RangeBounds<usize>) -> String {
    let start = char_range.start_bound();
    let end = char_range.end_bound();

    value.chars()
        .enumerate()
        .skip_while(|(i, _)| match start {
            std::ops::Bound::Included(start) => *i < *start,
            std::ops::Bound::Excluded(start) => *i <= *start,
            std::ops::Bound::Unbounded => false,
        })
        .take_while(|(i, _)| match end {
            std::ops::Bound::Included(end) => *i <= *end,
            std::ops::Bound::Excluded(end) => *i < *end,
            std::ops::Bound::Unbounded => true,
        })
        .map(|(_, ch)| ch)
        .collect()
}
    
pub fn pick_clean_head_token_sets(statements: &[SyntaxNode], is_empry_replacement: bool) -> Option<impl Iterator<Item = SyntaxTokenSet>> {
    if is_empry_replacement && (statements.len() <= 1) {
        return None;
    }

    let start_token_set = support::find_first_token_set(statements.last());

    let end_stmt = match is_empry_replacement {
        true => statements.last(),
        false => statements.first(),
    };
    let end_token_set = support::find_last_token_set(end_stmt);
    let sentinel = support::find_next_token_set(end_token_set.as_ref(), None);

    Some(std::iter::successors(start_token_set, move |token_set| support::find_next_token_set(Some(token_set), sentinel.as_ref())))
}

pub fn pick_clean_tail_token_sets(statements: &[SyntaxNode], is_empry_replacement: bool) -> Option<impl Iterator<Item = SyntaxTokenSet>> {
    let start_stmt = match is_empry_replacement {
        true if statements.len() > 1 => statements.get(1), 
        true => statements.first(), // pick EOF statment
        false => statements.first(),
    };
    let start_token_set = support::find_first_token_set(start_stmt);

    let end_token_set = support::find_last_token_set(statements.last());
    let sentinel = support::find_next_token_set(end_token_set.as_ref(), None);

    Some(std::iter::successors(start_token_set, move |token_set| support::find_next_token_set(Some(token_set), sentinel.as_ref())))
}

pub fn extract_clean_lookaheads<I: IntoIterator<Item = SyntaxTokenSet>>(token_sets: I, new_start_offset: Option<usize>, engine: &ScanningRuleSet, lookaheads: &mut VecDeque<Token>) {
    let mut iter = token_sets.into_iter().peekable();
    
    let offset_delta = match new_start_offset {
        Some(start_offset) => {
            iter.peek()
            .map(|token_set| start_offset as isize - token_set.metadata_key().offset as isize)
            .unwrap_or_default()
        }
        None => 0
    };

    let clean_tokensd = iter.filter_map(|token_set| into_lookahead(&token_set, engine, offset_delta));
    lookaheads.extend(clean_tokensd);
}

fn into_lookahead(token_set: &SyntaxTokenSet, engine: &ScanningRuleSet, offset_delta: isize) -> Option<Token> {
    let metadata = token_set.metadata();

    if ! [PatchAction::None, PatchAction::Delete, PatchAction::Invalid].contains(&metadata.patch) { return None }

    let (leadings, main, trailings) = 
        token_set.descendant_tokens()
        .fold((vec![], None, vec![]), |(mut leadings, mut main, mut trailings), item| {
            let key = item.metadata_key();
            let value = {
                let v = item.value();
                if v.is_empty() { None } else { Some(v.to_string()) }
            };
            let offset = if offset_delta < 0 { key.offset - offset_delta.abs() as usize } else { key.offset + offset_delta as usize };
            let event = ScanEvent{ kind: key.kind, offset, len: key.len, value };

            match item.metadata().node_type {
                parser_core::NodeType::Node | parser_core::NodeType::TokenSet => {}
                parser_core::NodeType::TokenItem => {
                    main = Some(event);
                }
                parser_core::NodeType::LeadingToken => {
                    leadings.push(event);
                }
                parser_core::NodeType::TrailingToken => {
                    trailings.push(event);
                }
            };
            (leadings, main, trailings)
        })
    ;

    Some(Token { 
        leading_trivia: if leadings.is_empty() { None } else { Some(leadings) }, 
        main: main.unwrap_or_else(|| {
            let key = token_set.metadata_key();
            ScanEvent { kind: engine.invalid(), offset: key.offset, len: key.len, value: None }
        }), 
        trailing_trivia: if trailings.is_empty() { None } else { Some(trailings) },
    })
}