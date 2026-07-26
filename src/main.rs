mod config;
mod herdr;
mod naming;
mod state;
mod sync;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "herdr-pane-name")]
struct Cli {
    #[command(subcommand)]
    command: CommandKind,
}

#[derive(Subcommand)]
enum CommandKind {
    Sync,
    Event,
    Hook { command: Option<String> },
    Reset,
    Clear,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = config::load()?;
    match cli.command {
        CommandKind::Sync | CommandKind::Event => sync::run(&cfg, false),
        CommandKind::Hook { command } => {
            sync::run(&cfg, false)?;
            if let Some(command) = command {
                eprintln!("herdr-pane-name: synced after {command}");
            }
            Ok(())
        }
        CommandKind::Reset => sync::run(&cfg, true),
        CommandKind::Clear => sync::clear(&cfg),
    }
}
