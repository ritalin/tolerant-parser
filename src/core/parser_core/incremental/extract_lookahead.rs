use crate::core::{engine_core::scanner_engine::{ScanEvent, ScanningRuleSet}, parser_core::{self, syntax_tree::{MetadataAccess, SyntaxTokenItem, SyntaxTokenSet}}, scanner_core::Token};

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

        start_offset = token_set.metadata_key().offset;
    };

    buf.push_str(text);

    if let Some(token_set) = end_dirty_token_set.as_ref() {
        token_set.descendant_tokens()
        .map(|token| (token.metadata().char_range(), token))
        .skip_while(|(char_range, _)| char_range.contains(&old_char_range.end))
        .filter_map(|(char_range, token)| match char_range {
            r if r.start > old_char_range.end => Some(token.value().to_string()),
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

pub fn into_lookahead(token_set: &SyntaxTokenSet, engine: &ScanningRuleSet, offset_delta: isize) -> crate::core::scanner_core::Token {
    let (leadings, main, trailings) = 
        token_set.descendant_tokens()
        .fold((vec![], None, vec![]), |(mut leadings, mut main, mut trailings), item| {
            let key = item.metadata_key();
            let value = {
                let v = item.value();
                if v.is_empty() { None } else { Some(v.to_string()) }
            };
            let offset = if offset_delta < 0 { key.offset - offset_delta as usize } else { key.offset + offset_delta as usize };
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