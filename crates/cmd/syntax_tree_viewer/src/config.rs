

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
pub struct CmdConfig {
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    pub input: String,
    #[arg(long)]
    pub no_color: bool,
}