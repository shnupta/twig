use crate::storage::{DataPaths, Storage};
use crate::utils::tree::{format_tree, TreeNode};
use anyhow::Result;

pub fn show_tree(assignee: Option<String>) -> Result<()> {
    let paths = DataPaths::new()?;
    let mut storage = Storage::new(paths.tasks_file().to_string_lossy().to_string());
    storage.load()?;

    // Filter by assignee if provided
    if let Some(ref a) = assignee {
        let filtered: Vec<_> = storage
            .get_all_tasks()
            .iter()
            .filter(|t| t.assigned_to.as_deref() == Some(a))
            .cloned()
            .collect();

        if filtered.is_empty() {
            println!("No tasks found for assignee: @{}", a);
            return Ok(());
        }

        // Create temporary storage with filtered tasks
        let mut temp_storage = Storage::new(String::new());
        *temp_storage.get_all_tasks_mut() = filtered;

        let forest = TreeNode::build_forest(&temp_storage);
        let lines = format_tree(&forest);

        if lines.is_empty() {
            println!("No tasks found.");
        } else {
            println!("\nTask Tree (filtered by @{}):", a);
            println!("{}", "=".repeat(60));
            for line in lines {
                println!("{}", line);
            }
            println!("{}", "=".repeat(60));
        }
    } else {
        let forest = TreeNode::build_forest(&storage);
        let lines = format_tree(&forest);

        if lines.is_empty() {
            println!("No tasks found.");
        } else {
            println!("\nTask Tree:");
            println!("{}", "=".repeat(60));
            for line in lines {
                println!("{}", line);
            }
            println!("{}", "=".repeat(60));
        }
    }

    Ok(())
}

