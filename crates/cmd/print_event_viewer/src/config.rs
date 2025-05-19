
#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
pub struct CmdConfig {
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    pub input: String,
    #[arg(long)]
    pub no_scan: bool,
    #[arg(long)]
    pub no_parse: bool,
    #[arg(long)]
    pub no_color: bool,
}


impl CmdConfig {
    pub fn to_capture_config(&self) -> parser_core::capture::EventCaptureConfig {
        parser_core::capture::EventCaptureConfig {
            no_scan: self.no_scan,
            no_parse: self.no_parse,
        }
    }
}
