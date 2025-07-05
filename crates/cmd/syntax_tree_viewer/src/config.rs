

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
pub struct CmdConfig {
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    pub input: String,
    #[arg(long)]
    pub no_color: bool,
    #[arg(long)]
    pub show_state: bool,
    #[arg(long)]
    pub enable_full_parse: bool,
    #[arg(long)]
    pub quiet: bool,
    #[arg(long)]
    pub parallel: bool,
    #[arg(long)]
    pub ignore_case: bool,
}