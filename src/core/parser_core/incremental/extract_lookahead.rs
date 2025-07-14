use std::collections::VecDeque;

use crate::core::{engine_core::scanner_engine::{ScanEvent, ScanningRuleSet}, parser_core::{self, incremental::support, syntax_tree::{MetadataAccess, SyntaxNode, SyntaxTokenItem, SyntaxTokenSet}}, scanner_core::Token};

pub fn extract_head_clean_lookaheads_forwards(start_token_set: Option<SyntaxTokenSet>, needle: usize, clean_token_sets: &mut VecDeque<SyntaxTokenSet>, dirty_tokenset: &mut Option<SyntaxTokenSet>) {
    let mut next_token_set = start_token_set;

    while let Some(token_set) = next_token_set {
        let char_range = token_set.metadata().char_range();

        match char_range {
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
        let char_range = token_set.metadata().char_range();

        match char_range {
            r if r.start > needle => {
                prev_token_set = support::find_prev_token_set(Some(&token_set), None);
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

pub fn concat_dirty_text(
    start_dirty_token_set: Option<&SyntaxTokenSet>, 
    text: &str, 
    end_dirty_token_set: Option<&SyntaxTokenSet>,
    old_char_range: std::ops::Range<usize>) -> (usize, String) 
{
    let start_dirty_size_hint = start_dirty_token_set.as_ref().map(|token_set| token_set.metadata_key().len).unwrap_or_default();
    let end_dirty_text_size_hint = end_dirty_token_set.as_ref().map(|token_set| token_set.metadata_key().len).unwrap_or_default();

    let mut buf = String::with_capacity(start_dirty_size_hint + text.len() + end_dirty_text_size_hint);
    let mut start_offset = old_char_range.start;

    if let Some(token_set) = start_dirty_token_set.as_ref() {
        token_set.descendant_tokens()
        .map(|token| (token.metadata().char_range(), token))
        .map_while(|(char_range, token)| match char_range {
            r if r.end <= old_char_range.start => Some(token.value().to_string()),
            r if r.contains(&old_char_range.start) => Some(substring_as_utf16_range(token, 0..(old_char_range.start - r.start))),
            _ => None,
        })
        .for_each(|s| buf.push_str(&s));

        start_offset -= buf.len();
    };

    buf.push_str(text);

    if let Some(token_set) = end_dirty_token_set.as_ref() {
        token_set.descendant_tokens()
        .map(|token| (token.metadata().char_range(), token))
        .filter_map(|(char_range, token)| match char_range {
            r if r.start >= old_char_range.end => Some(token.value().to_string()),
            r if r.contains(&old_char_range.end) => Some(substring_as_utf16_range(token, (old_char_range.end - r.start)..r.len())),
            _ => None,
        })
        .for_each(|s| buf.push_str(&s));
    };

    (start_offset, buf)
}

fn substring_as_utf16_range(token: SyntaxTokenItem, char_range: std::ops::Range<usize>) -> String {
    token.value().char_indices()
    .skip_while(|(i, _)| *i < char_range.start)
    .take_while(|(i, _)| *i < char_range.end)
    .map(|(_, ch)| ch)
    .collect()
}

pub fn pick_clean_head_token_sets(statements: &[SyntaxNode], sentinel: Option<SyntaxTokenSet>) -> impl Iterator<Item = SyntaxTokenSet> {
    let start_token_set = support::find_first_token_set(statements.first()).filter(|token_set| Some(token_set) != sentinel.as_ref());
    let end_token_set = support::find_last_token_set(statements.last());

    let sentinel = sentinel.or_else(|| support::find_next_token_set(end_token_set.as_ref(), None));
    std::iter::successors(start_token_set, move |token_set| support::find_next_token_set(Some(token_set), sentinel.as_ref()))
}

pub fn pick_clean_tail_token_sets(statements: &[SyntaxNode]) -> impl Iterator<Item = SyntaxTokenSet> {
    let start_token_set = support::find_first_token_set(statements.first());
    let end_token_set = support::find_last_token_set(statements.last());

    let sentinel = support::find_next_token_set(end_token_set.as_ref(), None);
    std::iter::successors(start_token_set, move |token_set| support::find_next_token_set(Some(token_set), sentinel.as_ref()))
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

    let clean_tokensd = iter.map(|token_set| into_lookahead(&token_set, engine, offset_delta));
    lookaheads.extend(clean_tokensd);
}

fn into_lookahead(token_set: &SyntaxTokenSet, engine: &ScanningRuleSet, offset_delta: isize) -> Token {
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

    Token { 
        leading_trivia: if leadings.is_empty() { None } else { Some(leadings) }, 
        main: main.unwrap_or_else(|| {
            let key = token_set.metadata_key();
            ScanEvent { kind: engine.invalid(), offset: key.offset, len: key.len, value: None }
        }), 
        trailing_trivia: if trailings.is_empty() { None } else { Some(trailings) },
    }
}