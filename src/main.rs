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
        Some(Commands::Start { id }) => {
            commands::start_task(id)?;
        }
        Some(Commands::Complete { id }) => {
            commands::complete_task(id)?;
        }
        Some(Commands::Cancel { id }) => {
            commands::cancel_task(id)?;
        }
        Some(Commands::Pause { id }) => {
            commands::pause_task(id)?;
        }
        Some(Commands::List { status, tag }) => {
            commands::list_tasks(status, tag)?;
        }
        Some(Commands::Show { id }) => {
            commands::show_task(id)?;
        }
        Some(Commands::Tree) => {
            commands::show_tree()?;
        }
        Some(Commands::Update {
            id,
            title,
            description,
            estimate,
            eta,
        }) => {
            commands::update_task(id, title, description, estimate, eta)?;
        }
        Some(Commands::Delete { id }) => {
            commands::delete_task(id)?;
        }
        Some(Commands::Tag { id, tags }) => {
            commands::tag_task(id, tags)?;
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
