use crate::models::Task;
use crate::storage::{DataPaths, Storage};
use crate::utils::parse_date;
use anyhow::{Context, Result};
use uuid::Uuid;

pub fn add_task(
    title: String,
    parent: Option<String>,
    tags: Option<String>,
    estimate: Option<String>,
    eta: Option<String>,
    assignee: Option<String>,
    description: Option<String>,
) -> Result<()> {
    let paths = DataPaths::new()?;
    let mut storage = Storage::new(paths.tasks_file().to_string_lossy().to_string());
    storage.load()?;

    let mut task = Task::new(title);

    // Set description
    if let Some(desc) = description {
        task.description = desc;
    }

    // Set parent
    if let Some(parent_id) = parent {
        let parent_uuid = if parent_id.len() == 8 {
            // Short ID
            storage
                .find_task_by_short_id(&parent_id)
                .map(|t| t.id)
                .context("Parent task not found")?
        } else {
            Uuid::parse_str(&parent_id).context("Invalid parent UUID")?
        };
        task.parent_id = Some(parent_uuid);
    }

    // Set tags
    if let Some(tags_str) = tags {
        task.tags = tags_str.split(',').map(|s| s.trim().to_string()).collect();
    }

    // Set estimate
    if let Some(est) = estimate {
        task.set_estimate(&est)?;
    }

    // Set ETA
    if let Some(eta_str) = eta {
        task.eta = Some(parse_date(&eta_str)?);
    }

    // Set assignee
    if let Some(assignee_name) = assignee {
        task.assigned_to = Some(assignee_name);
    }

    println!("✓ Task created: {} [{}]", task.title, task.short_id());
    storage.add_task(task)?;

    Ok(())
}

