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
        config::SubcommdSetting::Attach { engine, extension, path } => {
            commands::run_attach_provider(rt, config, &engine, &extension, path.as_path())
        }
        config::SubcommdSetting::Detach { engine } => {
            commands::run_detach_provider(rt, config, &engine)
        }
        config::SubcommdSetting::Inspect(setting) => {
            commands::run_inspect(rt, config, setting)
        }
        config::SubcommdSetting::List => {
            commands::run_list(rt, config)
        }
    }
}
