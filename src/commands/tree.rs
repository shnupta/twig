use crate::storage::{DataPaths, Storage};
use crate::utils::tree::{format_tree, TreeNode};
use anyhow::Result;

pub fn show_tree() -> Result<()> {
    let paths = DataPaths::new()?;
    let mut storage = Storage::new(paths.tasks_file().to_string_lossy().to_string());
    storage.load()?;

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

    Ok(())
}
