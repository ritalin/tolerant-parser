mod config;
mod runtime;
mod commands;

wasmtime::component::bindgen!({
    path: "./wit",
    world: "app-world",
});

pub fn main() -> Result<(), anyhow::Error> {
    let cli_setting = {
        use clap::Parser;
        config::CliSetting::parse()
    };

    let rt = runtime::Runtime::create()?;
    let config = config::ProviderConfig::load_default()?;

    match cli_setting.command {
        config::SubcommdSetting::Set { engine, extension, path } => {
            commands::run_install_provider(rt, config, &engine, &extension, path.as_path())
        }
        config::SubcommdSetting::Drop { engine } => {
            commands::run_drop_provider(rt, config, &engine)
        }
        config::SubcommdSetting::Capture(setting) => {
            commands::run_capture(rt, config, setting)
        }
        config::SubcommdSetting::List => {
            commands::run_list(rt, config)
        }
    }
}
