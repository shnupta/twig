use crate::commands::interactive::select_task_mut;
use crate::models::{Task, TaskStatus};
use crate::storage::{DataPaths, Storage};
use crate::utils::{format_datetime, parse_date};
use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, Confirm};
use uuid::Uuid;

fn resolve_task_id(storage: &Storage, id_str: &str) -> Result<Uuid> {
    if id_str.len() == 8 {
        storage
            .find_task_by_short_id(id_str)
            .map(|t| t.id)
            .context("Task not found")
    } else {
        Uuid::parse_str(id_str).context("Invalid UUID")
    }
}

pub fn start_task(id: Option<String>) -> Result<()> {
    let paths = DataPaths::new()?;
    let mut storage = Storage::new(paths.tasks_file().to_string_lossy().to_string());
    storage.load()?;

    let task_id = if let Some(id_str) = id {
        resolve_task_id(&storage, &id_str)?
    } else {
        // Interactive selection
        let tasks: Vec<Task> = storage
            .get_all_tasks()
            .iter()
            .filter(|t| t.status != TaskStatus::Completed && t.status != TaskStatus::Cancelled)
            .cloned()
            .collect();

        if let Some(task) = select_task_mut(&mut tasks.clone(), "Select task to start")? {
            task.id
        } else {
            return Ok(());
        }
    };

    if let Some(task) = storage.get_task_mut(task_id) {
        task.start();
        println!("✓ Started task: {} [{}]", task.title, task.short_id());
        storage.save()?;
    } else {
        anyhow::bail!("Task not found");
    }

    Ok(())
}

pub fn complete_task(id: Option<String>) -> Result<()> {
    let paths = DataPaths::new()?;
    let mut storage = Storage::new(paths.tasks_file().to_string_lossy().to_string());
    storage.load()?;

    let task_id = if let Some(id_str) = id {
        resolve_task_id(&storage, &id_str)?
    } else {
        // Interactive selection
        let tasks: Vec<Task> = storage
            .get_all_tasks()
            .iter()
            .filter(|t| t.status != TaskStatus::Completed && t.status != TaskStatus::Cancelled)
            .cloned()
            .collect();

        if let Some(task) = select_task_mut(&mut tasks.clone(), "Select task to complete")? {
            task.id
        } else {
            return Ok(());
        }
    };

    if let Some(task) = storage.get_task_mut(task_id) {
        task.complete();
        println!("✓ Completed task: {} [{}]", task.title, task.short_id());
        if task.total_time_seconds > 0 {
            println!("  Total time: {}", task.get_formatted_total_time());
        }
        storage.save()?;
    } else {
        anyhow::bail!("Task not found");
    }

    Ok(())
}

pub fn cancel_task(id: Option<String>) -> Result<()> {
    let paths = DataPaths::new()?;
    let mut storage = Storage::new(paths.tasks_file().to_string_lossy().to_string());
    storage.load()?;

    let task_id = if let Some(id_str) = id {
        resolve_task_id(&storage, &id_str)?
    } else {
        // Interactive selection
        let tasks: Vec<Task> = storage
            .get_all_tasks()
            .iter()
            .filter(|t| t.status != TaskStatus::Completed && t.status != TaskStatus::Cancelled)
            .cloned()
            .collect();

        if let Some(task) = select_task_mut(&mut tasks.clone(), "Select task to cancel")? {
            task.id
        } else {
            return Ok(());
        }
    };

    if let Some(task) = storage.get_task_mut(task_id) {
        task.cancel();
        println!("✓ Cancelled task: {} [{}]", task.title, task.short_id());
        storage.save()?;
    } else {
        anyhow::bail!("Task not found");
    }

    Ok(())
}

pub fn pause_task(id: Option<String>) -> Result<()> {
    let paths = DataPaths::new()?;
    let mut storage = Storage::new(paths.tasks_file().to_string_lossy().to_string());
    storage.load()?;

    let task_id = if let Some(id_str) = id {
        resolve_task_id(&storage, &id_str)?
    } else {
        // Interactive selection - show only tasks with active time entries
        let tasks: Vec<Task> = storage
            .get_all_tasks()
            .iter()
            .filter(|t| t.has_active_time_entry())
            .cloned()
            .collect();

        if tasks.is_empty() {
            println!("No tasks with active time tracking.");
            return Ok(());
        }

        if let Some(task) = select_task_mut(&mut tasks.clone(), "Select task to pause")? {
            task.id
        } else {
            return Ok(());
        }
    };

    if let Some(task) = storage.get_task_mut(task_id) {
        if !task.has_active_time_entry() {
            println!("Task has no active time tracking.");
            return Ok(());
        }
        task.pause();
        println!("✓ Paused task: {} [{}]", task.title, task.short_id());
        println!("  Total time: {}", task.get_formatted_total_time());
        storage.save()?;
    } else {
        anyhow::bail!("Task not found");
    }

    Ok(())
}

pub fn show_task(id: String) -> Result<()> {
    let paths = DataPaths::new()?;
    let mut storage = Storage::new(paths.tasks_file().to_string_lossy().to_string());
    storage.load()?;

    let task_id = resolve_task_id(&storage, &id)?;
    let task = storage.get_task(task_id).context("Task not found")?;

    println!("\n{}", "=".repeat(60));
    println!("Task: {}", task.title);
    println!("{}", "=".repeat(60));
    println!("ID:          {}", task.id);
    println!("Short ID:    {}", task.short_id());
    println!(
        "Status:      {}",
        match task.status {
            TaskStatus::NotStarted => "○ Not Started",
            TaskStatus::InProgress => "◐ In Progress",
            TaskStatus::Completed => "● Completed",
            TaskStatus::Cancelled => "✗ Cancelled",
        }
    );

    if !task.description.is_empty() {
        println!("Description: {}", task.description);
    }

    if let Some(ref assignee) = task.assigned_to {
        println!("Assignee:    @{}", assignee);
    }

    if !task.tags.is_empty() {
        println!(
            "Tags:        {}",
            task.tags.iter().map(|t| format!("#{}", t)).collect::<Vec<_>>().join(" ")
        );
    }

    if let Some(estimate) = task.get_formatted_estimate() {
        println!("Estimate:    {}", estimate);
    }

    if let Some(eta) = task.eta {
        println!("ETA:         {}", format_datetime(&eta));
    }

    println!("Created:     {}", format_datetime(&task.created_at));

    if let Some(started) = task.started_at {
        println!("Started:     {}", format_datetime(&started));
    }

    if let Some(completed) = task.completed_at {
        println!("Completed:   {}", format_datetime(&completed));
    }

    if let Some(cancelled) = task.cancelled_at {
        println!("Cancelled:   {}", format_datetime(&cancelled));
    }

    if task.total_time_seconds > 0 {
        println!("Total Time:  {}", task.get_formatted_total_time());
    }

    // Show hierarchy
    let hierarchy = storage.get_task_hierarchy(task);
    if hierarchy.len() > 1 {
        println!("\nHierarchy:");
        for (i, id) in hierarchy.iter().enumerate() {
            if let Some(t) = storage.get_task(*id) {
                println!("  {}{}", "  ".repeat(i), t.title);
            }
        }
    }

    // Show children
    let children = storage.get_children(task.id);
    if !children.is_empty() {
        println!("\nSubtasks:");
        for child in children {
            let status_icon = match child.status {
                TaskStatus::NotStarted => "○",
                TaskStatus::InProgress => "◐",
                TaskStatus::Completed => "●",
                TaskStatus::Cancelled => "✗",
            };
            println!("  {} {} [{}]", status_icon, child.title, child.short_id());
        }
    }

    println!("{}", "=".repeat(60));

    Ok(())
}

pub fn update_task(
    id: String,
    title: Option<String>,
    description: Option<String>,
    estimate: Option<String>,
    eta: Option<String>,
    assignee: Option<String>,
) -> Result<()> {
    let paths = DataPaths::new()?;
    let mut storage = Storage::new(paths.tasks_file().to_string_lossy().to_string());
    storage.load()?;

    let task_id = resolve_task_id(&storage, &id)?;

    if let Some(task) = storage.get_task_mut(task_id) {
        let mut updated = false;

        if let Some(new_title) = title {
            task.title = new_title;
            updated = true;
        }

        if let Some(new_desc) = description {
            task.description = new_desc;
            updated = true;
        }

        if let Some(est) = estimate {
            task.set_estimate(&est)?;
            updated = true;
        }

        if let Some(eta_str) = eta {
            task.eta = Some(parse_date(&eta_str)?);
            updated = true;
        }

        if let Some(new_assignee) = assignee {
            task.assigned_to = Some(new_assignee);
            updated = true;
        }

        if updated {
            println!("✓ Task updated: {} [{}]", task.title, task.short_id());
            storage.save()?;
        } else {
            println!("No changes made.");
        }
    } else {
        anyhow::bail!("Task not found");
    }

    Ok(())
}

pub fn delete_task(id: String) -> Result<()> {
    let paths = DataPaths::new()?;
    let mut storage = Storage::new(paths.tasks_file().to_string_lossy().to_string());
    storage.load()?;

    let task_id = resolve_task_id(&storage, &id)?;
    let task = storage.get_task(task_id).context("Task not found")?;

    // Check for children
    let children = storage.get_children(task_id);
    if !children.is_empty() {
        println!("Warning: This task has {} subtask(s).", children.len());
    }

    let confirmation = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Delete task '{}' [{}]?", task.title, task.short_id()))
        .default(false)
        .interact()?;

    if confirmation {
        storage.delete_task(task_id)?;
        println!("✓ Task deleted");
    } else {
        println!("Cancelled");
    }

    Ok(())
}

pub fn tag_task(id: String, tags: Vec<String>) -> Result<()> {
    let paths = DataPaths::new()?;
    let mut storage = Storage::new(paths.tasks_file().to_string_lossy().to_string());
    storage.load()?;

    let task_id = resolve_task_id(&storage, &id)?;

    if let Some(task) = storage.get_task_mut(task_id) {
        for tag in tags {
            if !task.tags.contains(&tag) {
                task.tags.push(tag.clone());
                println!("✓ Added tag: #{}", tag);
            } else {
                println!("  Tag already exists: #{}", tag);
            }
        }
        storage.save()?;
    } else {
        anyhow::bail!("Task not found");
    }

    Ok(())
}

