mod audit;
mod cli;
mod config;
#[allow(dead_code)]
mod config_file;
mod env;
mod git;
mod logging;
mod plugin;
mod sync;
#[allow(dead_code)]
mod telemetry;
pub mod tui;
pub mod vault;
#[allow(dead_code)]
mod webhooks;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli::execute(cli)
}
