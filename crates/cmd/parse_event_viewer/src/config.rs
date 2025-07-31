// use tolerant_parser_sdk::core::engine_core::scanner_engine::CaseSensitivity;
// use tolerant_parser_sdk::core::parser_core::{self, ParseMode};

use std::collections::HashMap;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
pub struct CliSetting {
    #[command(subcommand)]
    pub command: SubcommdSetting,
}

#[derive(clap::Subcommand, Debug)]
pub enum SubcommdSetting {
    Set { 
        #[arg(short = 'm', value_name = "ENGINE")]
        engine: String, 
        #[arg(short = 'e', value_name = "EXT")]
        extension: String, 
        #[arg(short = 'f', long = "file", value_name = "WASI-FILE")]
        path: std::path::PathBuf 
    },
    Drop { 
        #[arg(short = 'm', value_name = "ENGINEx")]
        engine: String 
    },
    Capture(CaptureSetting),
    List,
}

#[derive(clap::Args, Debug)]
pub struct CaptureSetting {
    #[arg(short = 'm', value_name = "ENGINE")]
    pub engine: Option<String>, 
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    pub input: std::path::PathBuf,
    #[arg(long)]
    pub no_scan: bool,
    #[arg(long)]
    pub no_parse: bool,
    #[arg(long)]
    pub no_color: bool,
    #[arg(long)]
    pub quiet: bool,
    #[arg(long)]
    pub ignore_case: bool,
}


// impl CmdConfig {
//     pub fn to_capture_config(&self) -> parser_core::capture::EventCaptureConfig {
//         parser_core::capture::EventCaptureConfig {
//             mode: if self.enable_full_parse { ParseMode::Full } else { ParseMode::ByStatement },
//             no_scan: self.no_scan,
//             no_parse: self.no_parse,
//             case_sensitive: if self.ignore_case { CaseSensitivity::Insensitive } else { CaseSensitivity::Sensitive },
            
//         }
//     }
// }

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ProviderConfig {
    config_path: std::path::PathBuf,
    providers: HashMap<String, ProviderConfigItem>,
}

impl ProviderConfig {
    pub fn load_default() -> Result<ProviderConfig, anyhow::Error> {
        let Some(dirs) = directories::ProjectDirs::from("", "", env!("CARGO_PKG_NAME"))
        else {
            anyhow::bail!("Can not load configlation.");
        };
        
        let file_path = dirs.config_dir().to_path_buf().join("providers.toml");
        if let Some(dir_path) = file_path.parent() {
            std::fs::create_dir_all(dir_path)?;
        }

        let content = std::fs::read_to_string(file_path.as_path()).unwrap_or_else(|_| "".to_string());
        let providers = toml::from_str::<HashMap<String, ProviderConfigItem>>(&content)?;

        Ok(ProviderConfig { config_path: file_path, providers })
    }

    pub fn save(&self) -> Result<(), anyhow::Error> {
        let content = toml::to_string_pretty(&self.providers)?;

        if let Some(dir_path) = self.config_path.parent() {
            std::fs::create_dir_all(dir_path)?;
        }
        std::fs::write(self.config_path.as_path(), &content)?;

        Ok(())
    }

    fn get_by_engine(&self, engine: &str) -> Option<&ProviderConfigItem> {
        self.providers.get(engine)
    }

    fn get_by_extension(&self, path: &str) -> Vec<(String, &ProviderConfigItem)> {
        self.providers.iter()
            .filter(|(_, p)| path.ends_with(&p.extension))
            .map(|(engine, p)| (engine.to_string(), p))
            .collect()
    }

    pub fn put(&mut self, engine: &str, ext: &str) -> Result<&ProviderConfigItem, anyhow::Error> {
        let Some(config_dir_path) = self.config_path.parent() else {
            anyhow::bail!("Configration is not found.");
        };

        let provider = self.providers
            .entry(engine.to_string())
            .or_insert_with(|| {
                let aot_path = config_dir_path.join("aot").join(format!("{}_capture_provider.cwasm", engine));
                ProviderConfigItem{ extension: ext.into(), path: aot_path.into() }
            })
        ;

        Ok(provider)
    }

    pub fn resolve(&self, engine: Option<&String>, path: &std::path::Path) -> Result<&ProviderConfigItem, anyhow::Error> {
        if let Some(engine) = engine {
            if let Some(provider) = self.get_by_engine(engine) {
                return Ok(provider);
            }
        }

        let candidates = match self.get_by_extension(&path.display().to_string()) {
            candidates if candidates.len() == 0 => {
                anyhow::bail!("Can not resolve provider");
            }
            candidates if candidates.len() > 1 => {
                let msg = 
                    vec!["many candidates found:".to_string()].into_iter()
                    .chain(candidates.iter().map(|(engine, p)| format!("    {} ({})", engine, p.extension)))
                    .collect::<Vec<_>>()
                ;
                anyhow::bail!("{}", msg.join("\n"));
            }
            candidates => candidates
        };
        
        Ok(candidates.first().map(|(_, p)| p).unwrap())
    }

    pub fn remove_by_engine(&mut self, engine: &str) -> Option<ProviderConfigItem> {
        self.providers.remove(engine)
    }

    pub fn providers(&self) -> impl Iterator<Item = (&String, &ProviderConfigItem)> {
        self.providers.iter()
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ProviderConfigItem {
    pub extension: String,
    pub path: std::path::PathBuf,
}
