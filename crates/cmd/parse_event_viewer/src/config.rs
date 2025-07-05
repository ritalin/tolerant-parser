use engine_core::scanner_engine::CaseSensitivity;
use parser_core::ParseMode;


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
    #[arg(long)]
    pub quiet: bool,
    #[arg(long)]
    pub enable_full_parse: bool,
    #[arg(long)]
    pub ignore_case: bool,
}


impl CmdConfig {
    pub fn to_capture_config(&self) -> parser_core::capture::EventCaptureConfig {
        parser_core::capture::EventCaptureConfig {
            mode: if self.enable_full_parse { ParseMode::Full } else { ParseMode::ByStatement },
            no_scan: self.no_scan,
            no_parse: self.no_parse,
            case_sensitive: if self.ignore_case { CaseSensitivity::Insensitive } else { CaseSensitivity::Sensitive },
        }
    }
}
