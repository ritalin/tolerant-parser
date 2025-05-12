use clap::Parser;

fn main() -> Result<(), anyhow::Error> {
    let config = gen_transtion_table::CmdConfig::parse();
    
    gen_transtion_table::generate(config)?;

    Ok(())
}
