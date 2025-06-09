use parser_core::{syntax_tree::{MetadataAccess, SyntaxNode, SyntaxTokenSet}, NodeMetadata, NodeMetadataKey, PatchAction};

pub mod config;

pub fn print_tree(source: &str, root_node: &SyntaxNode, config: config::CmdConfig) {
    if config.quiet { return }

    println!("--------------------------------------------------------------------------------");
    println!("`{source}`");
    println!("--------------------------------------------------------------------------------");
    println!();

    print_tree_internal(root_node, &config, 0, 0);
}

pub fn print_parael_statement(mut statements: parser_core::paralell::Statements, config: config::CmdConfig) {
    if config.quiet { return }
    
    statements.members.sort_by(|lhs, rhs| lhs.seq().cmp(&rhs.seq()));

    for stmt in statements.members {
        println!();
        println!("--------------------------------------------------------------------------------");
        println!("#{:<04}", stmt.seq()+1);
        println!("`{stmt}`");
        println!("--------------------------------------------------------------------------------");

        let adjusted_byte_offset = stmt.byte_offset();
        let node = stmt.into_root(statements.engine);
        print_tree_internal(&node, &config, adjusted_byte_offset, 1);
    }
}

fn print_tree_internal(parent: &SyntaxNode, config: &config::CmdConfig, adjusted_byte_offset: usize, indent_level: usize) {
    print_node(parent.metadata_key().into_global(adjusted_byte_offset), parent.metadata(), None, config, indent_level);

    for child in parent.children() {
        match child {
            parser_core::syntax_tree::SyntaxElementDef::Node(node) => {
                print_tree_internal(&node, config, adjusted_byte_offset, indent_level + 1);
            }
            parser_core::syntax_tree::SyntaxElementDef::TokenSet(token_set) => {
                print_token_set(&token_set, config, adjusted_byte_offset, indent_level + 1)
            }
        }
    }
}

fn print_token_set(token_set: &SyntaxTokenSet, config: &config::CmdConfig, adjusted_byte_offset: usize, indent_level: usize) {
    print_node(token_set.metadata_key().into_global(adjusted_byte_offset), token_set.metadata(), None, config, indent_level);

    for item in token_set.leading_trivia() {
        print_node(item.metadata_key().into_global(adjusted_byte_offset), item.metadata(), Some(item.value()), config, indent_level + 1);
    }

    let item = token_set.token();
    print_node(item.metadata_key().into_global(adjusted_byte_offset), item.metadata(), Some(item.value()), config, indent_level + 1);

    for item in token_set.trailing_trivia() {
        print_node(item.metadata_key().into_global(adjusted_byte_offset), item.metadata(), Some(item.value()), config, indent_level + 1);
    }
}

fn print_node(key: NodeMetadataKey, metadata: NodeMetadata, value: Option<&str>, config: &config::CmdConfig, indent_level: usize) {
    let range_str = format!("({}-{})", key.offset, key.offset + key.len);
    let node_type_str = match metadata.patch {
        PatchAction::None => metadata.node_type.to_string(),
        PatchAction::Delete | PatchAction::Shift | PatchAction::Invalid => {
            format!("{}(patch: {})", metadata.node_type.to_string(), metadata.patch.to_string())
        }
    };
    let state_str = if config.show_state { format!("{:>6}", metadata.edit_state) } else { "".to_string() };
    let value = value.map(|s| format!("{s:?}")).unwrap_or_default();

    let plain = format!(
        "{:<16}{:<30}{} | {:width$}{} {}", 
        range_str, node_type_str, state_str, "", key.kind.text, value, width = indent_level * 4
    );

    let colored = match (metadata.node_type.clone(), config.no_color) {
        (_, false) if metadata.patch == PatchAction::Invalid => {
            ansi_term::Color::Red.paint(plain).to_string()
        }
        (_, false) if metadata.patch != PatchAction::None => {
            ansi_term::Color::RGB(255, 165, 0).paint(plain).to_string()
        }
        (parser_core::NodeType::TokenSet, false) => {
            ansi_term::Color::Cyan.paint(plain).to_string()
        }
        (parser_core::NodeType::TokenItem, false) => {
            ansi_term::Color::Yellow.paint(plain).to_string()
        }
        (parser_core::NodeType::LeadingToken, false) |
        (parser_core::NodeType::TrailingToken, false) => {
            ansi_term::Color::RGB(128, 128, 128).paint(plain).to_string()
        }
        _ => plain
    };

    println!("{colored}")
}
