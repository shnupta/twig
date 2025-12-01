use crate::storage::{json_store, DataPaths};
use anyhow::Result;
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};

pub fn add_reportee(name: String) -> Result<()> {
    let paths = DataPaths::new()?;
    let mut config = json_store::load_config(&paths.config_file())?;

    if config.add_reportee(name.clone()) {
        json_store::save_config(&paths.config_file(), &config)?;

        // Create empty tasks file for reportee
        let reportee_path = paths.reportee_tasks_file(&name);
        if !reportee_path.exists() {
            std::fs::write(&reportee_path, "[]")?;
        }

        println!("✓ Added reportee: {}", name);
    } else {
        println!("Reportee already exists: {}", name);
    }

    Ok(())
}

pub fn list_reportees() -> Result<()> {
    let paths = DataPaths::new()?;
    let config = json_store::load_config(&paths.config_file())?;

    if config.reportees.is_empty() {
        println!("No reportees configured.");
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Name", "Tasks File"]);

    for reportee in &config.reportees {
        let file_path = paths.reportee_tasks_file(reportee);
        let exists = if file_path.exists() { "✓" } else { "✗" };
        table.add_row(vec![
            reportee,
            &format!("{} {}", exists, file_path.display()),
        ]);
    }

    println!("{}", table);
    println!("\nTotal: {} reportee(s)", config.reportees.len());

    Ok(())
}

pub fn remove_reportee(name: String) -> Result<()> {
    let paths = DataPaths::new()?;
    let mut config = json_store::load_config(&paths.config_file())?;

    if config.remove_reportee(&name) {
        json_store::save_config(&paths.config_file(), &config)?;
        println!("✓ Removed reportee: {}", name);
        println!("Note: Tasks file for {} was not deleted.", name);
    } else {
        println!("Reportee not found: {}", name);
    }

    Ok(())
}
