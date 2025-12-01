use crate::models::{Task, TaskStatus};
use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Select};

pub fn select_task<'a>(tasks: &'a [Task], prompt: &str) -> Result<Option<&'a Task>> {
    if tasks.is_empty() {
        println!("No tasks available.");
        return Ok(None);
    }

    let items: Vec<String> = tasks
        .iter()
        .map(|t| {
            let status_icon = match t.status {
                TaskStatus::NotStarted => "○",
                TaskStatus::InProgress => "◐",
                TaskStatus::Completed => "●",
                TaskStatus::Cancelled => "✗",
            };
            format!("{} {} [{}]", status_icon, t.title, t.short_id())
        })
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(&items)
        .default(0)
        .interact_opt()?;

    Ok(selection.map(|i| &tasks[i]))
}

pub fn select_task_mut<'a>(tasks: &'a mut [Task], prompt: &str) -> Result<Option<&'a mut Task>> {
    if tasks.is_empty() {
        println!("No tasks available.");
        return Ok(None);
    }

    let items: Vec<String> = tasks
        .iter()
        .map(|t| {
            let status_icon = match t.status {
                TaskStatus::NotStarted => "○",
                TaskStatus::InProgress => "◐",
                TaskStatus::Completed => "●",
                TaskStatus::Cancelled => "✗",
            };
            format!("{} {} [{}]", status_icon, t.title, t.short_id())
        })
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(&items)
        .default(0)
        .interact_opt()?;

    Ok(selection.map(|i| &mut tasks[i]))
}
