mod cli;
mod commands;
mod models;
mod storage;
mod tui;
mod utils;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};
use cli::{Cli, Commands, ReporteeCommands};
use std::io;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => {
            // No command provided - launch TUI
            tui::run_tui()?;
        }
        Some(Commands::Add {
            title,
            parent,
            tags,
            estimate,
            eta,
            description,
        }) => {
            commands::add_task(title, parent, tags, estimate, eta, description)?;
        }
        Some(Commands::Start) => {
            commands::start_task()?;
        }
        Some(Commands::Complete) => {
            commands::complete_task()?;
        }
        Some(Commands::Cancel) => {
            commands::cancel_task()?;
        }
        Some(Commands::Pause) => {
            commands::pause_task()?;
        }
        Some(Commands::List { status, tag }) => {
            commands::list_tasks(status, tag)?;
        }
        Some(Commands::Show) => {
            commands::show_task()?;
        }
        Some(Commands::Tree) => {
            commands::show_tree()?;
        }
        Some(Commands::Update {
            title,
            description,
            estimate,
            eta,
        }) => {
            commands::update_task(title, description, estimate, eta)?;
        }
        Some(Commands::Delete) => {
            commands::delete_task()?;
        }
        Some(Commands::Tag { tags }) => {
            commands::tag_task(tags)?;
        }
        Some(Commands::Reportee { command }) => match command {
            ReporteeCommands::Add { name } => {
                commands::add_reportee(name)?;
            }
            ReporteeCommands::List => {
                commands::list_reportees()?;
            }
            ReporteeCommands::Remove { name } => {
                commands::remove_reportee(name)?;
            }
        },
        Some(Commands::Report { period, date }) => {
            commands::generate_report(period, date)?;
        }
        Some(Commands::Stats { period, date }) => {
            commands::show_stats(period, date)?;
        }
        Some(Commands::Tui) => {
            tui::run_tui()?;
        }
        Some(Commands::Completions { shell }) => {
            generate_completions(shell);
        }
    }

    Ok(())
}

fn generate_completions(shell: Shell) {
    let mut cmd = Cli::command();
    let bin_name = "twig";

    eprintln!("Generating completion file for {shell}...");
    generate(shell, &mut cmd, bin_name, &mut io::stdout());
}
