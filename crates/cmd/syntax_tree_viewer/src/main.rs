use engine_core::scanner_engine::CaseSensitivity;
use parser_core::{error_recovery::RecoveryPenalty, ParseMode};
use syntax_tree_viewer::{print_tree, print_parallel_statement, config};

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
        case_sensitive: if cmd_config.ignore_case { CaseSensitivity::Insensitive } else { CaseSensitivity::Sensitive }
    };

    match cmd_config.parallel {
        false => {
            let parser = parser_core::Parser::new(engine, parse_config);
            let tree = parser.parse(&source)?;
            print_tree(&source, &tree.root(), cmd_config);
        }
        true => {
            let parser = parser_core::paralell::Parser::new(engine);
            let statements = parser.parse_with_config(&source, parse_config)?;
            print_parallel_statement(statements, cmd_config);
        }
    }

    Ok(())
}
