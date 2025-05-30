use config::CmdConfig;
use parser_core::{error_recovery::RecoveryPenalty, syntax_tree::{MetadataAccess, SyntaxNode, SyntaxTokenSet}, NodeMetadata, NodeMetadataKey, ParseMode, PatchAction};

mod config;

fn main() -> Result<(), anyhow::Error> {
    let cmd_config = {
        use clap::Parser;
        config::CmdConfig::parse()
    };

    let source = std::fs::read_to_string(cmd_config.input.clone())?;
    let engine = sqlite_engine::create()?;
    let parser = parser_core::Parser::new(engine);
    let parse_config = parser_core::ParserConfig {
        mode: if cmd_config.enable_full_parse { ParseMode::Full } else { ParseMode::ByStatement },
        penalty: RecoveryPenalty::default(),
    };
    let tree = parser.parse_with_config(&source, parse_config)?;
    
    println!("`{}`", source);
    println!("--------------------------------------------------------------------------------");
    
    print_tree(&tree.root(), &cmd_config);
    Ok(())
}

fn print_tree(root_node: &SyntaxNode, config: &CmdConfig) {
    print_tree_internal(root_node, config, 0);
}

fn print_tree_internal(parent: &SyntaxNode, config: &CmdConfig, indent_level: usize) {
    print_node(&parent.metadata_key(), parent.metadata(), None, config, indent_level);

    for child in parent.children() {
        match child {
            parser_core::syntax_tree::SyntaxElementDef::Node(node) => {
                print_tree_internal(&node, config, indent_level + 1);
            }
            parser_core::syntax_tree::SyntaxElementDef::TokenSet(token_set) => {
                print_token_set(&token_set, config, indent_level + 1)
            }
        }
    }
}

fn print_token_set(token_set: &SyntaxTokenSet, config: &CmdConfig, indent_level: usize) {
    print_node(&token_set.metadata_key(), token_set.metadata(), None, config, indent_level);

    for item in token_set.leading_trivia() {
        print_node(&item.metadata_key(), item.metadata(), Some(item.value()), config, indent_level + 1);
    }

    let item = token_set.token();
    print_node(&item.metadata_key(), item.metadata(), Some(item.value()), config, indent_level + 1);

    for item in token_set.trailing_trivia() {
        print_node(&item.metadata_key(), item.metadata(), Some(item.value()), config, indent_level + 1);
    }
}

fn print_node(key: &NodeMetadataKey, metadata: &NodeMetadata, value: Option<&str>, config: &CmdConfig, indent_level: usize) {
    let range_str = format!("({}-{})", key.offset, key.offset + key.len);
    let node_type_str = match metadata.patch {
        PatchAction::None => metadata.node_type.to_string(),
        PatchAction::Delete | PatchAction::Shift | PatchAction::Invalid => {
            format!("{}(patch: {})", metadata.node_type.to_string(), metadata.patch.to_string())
        }
    };
    let value = value.map(|s| format!("{s:?}")).unwrap_or_default();

    let plain = format!(
        "{:<16}{:<30} | {:width$}{} {}", 
        range_str, node_type_str, "", key.kind.text, value, width = indent_level * 4
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
