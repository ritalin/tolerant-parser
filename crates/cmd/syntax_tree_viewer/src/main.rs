use parser_core::{error_recovery::RecoveryPenalty, ParseMode};
use syntax_tree_viewer::{print_tree, print_parael_statement, config};

fn main() -> Result<(), anyhow::Error> {
    let cmd_config = {
        use clap::Parser;
        config::CmdConfig::parse()
    };

    let source = std::fs::read_to_string(cmd_config.input.clone())?;
    let engine = sqlite_engine::create()?;
    let parse_config = parser_core::ParserConfig {
        mode: if cmd_config.enable_full_parse && (!cmd_config.parallel) { ParseMode::Full } else { ParseMode::ByStatement },
        penalty: RecoveryPenalty::default(),
    };

    match cmd_config.parallel {
        false => {
            let parser = parser_core::Parser::new(engine);
            let tree = parser.parse_with_config(&source, parse_config)?;
            print_tree(&source, &tree.root(), cmd_config);
        }
        true => {
            let parser = parser_core::paralell::Parser::new(engine);
            let statements = parser.parse_with_config(&source, parse_config)?;
            print_parael_statement(statements, cmd_config);
        }
    }

    Ok(())
}
