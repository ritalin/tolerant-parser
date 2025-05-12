
pub fn tokens_to_string(tokens: proc_macro2::TokenStream, depth: usize) -> String {
    let mut s = String::new();
    s.push_str(&"  ".repeat(depth));
    s.push_str(&tokens.to_string());
    s
}

pub fn with_indent(token_str: &str, depth: usize) -> String {
    let mut s = String::new();
    s.push_str(&"  ".repeat(depth));
    s.push_str(token_str);
    s
}
