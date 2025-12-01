use crate::cli::StatusFilter;
use crate::models::{Task, TaskStatus};
use crate::storage::{DataPaths, Storage};
use crate::utils::format_datetime;
use anyhow::Result;
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};

pub fn list_tasks(
    status: Option<StatusFilter>,
    tag: Option<String>,
    assignee: Option<String>,
) -> Result<()> {
    let paths = DataPaths::new()?;
    let mut storage = Storage::new(paths.tasks_file().to_string_lossy().to_string());
    storage.load()?;

    let tasks = storage.get_all_tasks();
    let filtered: Vec<&Task> = tasks
        .iter()
        .filter(|task| {
            if let Some(ref s) = status {
                let task_status = match s {
                    StatusFilter::NotStarted => TaskStatus::NotStarted,
                    StatusFilter::InProgress => TaskStatus::InProgress,
                    StatusFilter::Completed => TaskStatus::Completed,
                    StatusFilter::Cancelled => TaskStatus::Cancelled,
                };
                if task.status != task_status {
                    return false;
                }
            }
            if let Some(ref t) = tag {
                if !task.tags.iter().any(|tag| tag == t) {
                    return false;
                }
            }
            if let Some(ref a) = assignee {
                if task.assigned_to.as_deref() != Some(a) {
                    return false;
                }
            }
            true
        })
        .collect();

    if filtered.is_empty() {
        println!("No tasks found.");
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["ID", "Status", "Title", "Tags", "Assignee", "Time", "Created"]);

    for task in &filtered {
        let status_str = match task.status {
            TaskStatus::NotStarted => Cell::new("○ Not Started").fg(Color::Grey),
            TaskStatus::InProgress => Cell::new("◐ In Progress").fg(Color::Yellow),
            TaskStatus::Completed => Cell::new("● Completed").fg(Color::Green),
            TaskStatus::Cancelled => Cell::new("✗ Cancelled").fg(Color::Red),
        };

        let tags_str = if task.tags.is_empty() {
            String::new()
        } else {
            task.tags.iter().map(|t| format!("#{}", t)).collect::<Vec<_>>().join(" ")
        };

        let assignee_str = task
            .assigned_to
            .as_ref()
            .map(|a| format!("@{}", a))
            .unwrap_or_default();

        let time_str = if task.total_time_seconds > 0 {
            task.get_formatted_total_time()
        } else {
            String::new()
        };

        table.add_row(vec![
            Cell::new(task.short_id()),
            status_str,
            Cell::new(&task.title),
            Cell::new(tags_str),
            Cell::new(assignee_str),
            Cell::new(time_str),
            Cell::new(format_datetime(&task.created_at)),
        ]);
    }

    println!("{}", table);
    println!("\nTotal: {} task(s)", filtered.len());

    Ok(())
}

