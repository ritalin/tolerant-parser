
/// Parser and scanner engine generator
#[derive(clap::Parser)]
#[command(version, long_about = None)]
pub struct CmdConfig {
    /// source direcroty path
    #[arg(long, short = 'd', value_name = "DIR")]
    pub source_dir: String,
    /// output dir path
    #[arg(long, short = 'o', value_name = "DIR")]
    pub output_dir: String,
    /// symbol definitin JSON file name
    #[arg(long, value_name = "FILE", default_value = "grammar_symbols.json")]
    pub grammar_symbol: String,
    /// scanning rule JSON file name
    #[arg(long, value_name = "FILE", default_value = "grammar_scan_rules.json")]
    pub grammar_scan_rule: String,
    /// parsing rule JSON file name
    #[arg(long, value_name = "FILE", default_value = "grammar_parse_rules.json")]
    pub grammar_parse_rule: String,
}