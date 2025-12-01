use crate::models::{Config, Task};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use uuid::Uuid;

pub struct Storage {
    tasks: Vec<Task>,
    file_path: String,
}

impl Storage {
    pub fn new(file_path: String) -> Self {
        Self {
            tasks: Vec::new(),
            file_path,
        }
    }

    pub fn load(&mut self) -> Result<()> {
        let path = Path::new(&self.file_path);
        if !path.exists() {
            // Initialize with empty tasks list
            self.tasks = Vec::new();
            return Ok(());
        }

        let content = fs::read_to_string(path)
            .context("Failed to read tasks file")?;
        
        if content.trim().is_empty() {
            self.tasks = Vec::new();
        } else {
            self.tasks = serde_json::from_str(&content)
                .context("Failed to parse tasks JSON")?;
        }
        
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.tasks)
            .context("Failed to serialize tasks")?;
        fs::write(&self.file_path, json)
            .context("Failed to write tasks file")?;
        Ok(())
    }

    pub fn add_task(&mut self, task: Task) -> Result<()> {
        self.tasks.push(task);
        self.save()
    }

    pub fn update_task(&mut self, task: Task) -> Result<()> {
        if let Some(pos) = self.tasks.iter().position(|t| t.id == task.id) {
            self.tasks[pos] = task;
            self.save()
        } else {
            Err(anyhow::anyhow!("Task not found"))
        }
    }

    pub fn delete_task(&mut self, id: Uuid) -> Result<()> {
        if let Some(pos) = self.tasks.iter().position(|t| t.id == id) {
            self.tasks.remove(pos);
            self.save()
        } else {
            Err(anyhow::anyhow!("Task not found"))
        }
    }

    pub fn get_task(&self, id: Uuid) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn get_task_mut(&mut self, id: Uuid) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    pub fn get_all_tasks(&self) -> &[Task] {
        &self.tasks
    }

    pub fn get_all_tasks_mut(&mut self) -> &mut Vec<Task> {
        &mut self.tasks
    }

    pub fn find_task_by_short_id(&self, short_id: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.short_id() == short_id)
    }

    pub fn find_task_by_short_id_mut(&mut self, short_id: &str) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.short_id() == short_id)
    }

    pub fn get_root_tasks(&self) -> Vec<&Task> {
        self.tasks.iter().filter(|t| t.parent_id.is_none()).collect()
    }

    pub fn get_children(&self, parent_id: Uuid) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|t| t.parent_id == Some(parent_id))
            .collect()
    }

    pub fn get_task_hierarchy(&self, task: &Task) -> Vec<Uuid> {
        let mut hierarchy = vec![task.id];
        let mut current_id = task.parent_id;
        
        while let Some(id) = current_id {
            hierarchy.insert(0, id);
            if let Some(parent) = self.get_task(id) {
                current_id = parent.parent_id;
            } else {
                break;
            }
        }
        
        hierarchy
    }
}

pub fn load_config(path: &Path) -> Result<Config> {
    if !path.exists() {
        let config = Config::default();
        save_config(path, &config)?;
        return Ok(config);
    }

    let content = fs::read_to_string(path)
        .context("Failed to read config file")?;
    let config = serde_json::from_str(&content)
        .context("Failed to parse config JSON")?;
    Ok(config)
}

pub fn save_config(path: &Path, config: &Config) -> Result<()> {
    let json = serde_json::to_string_pretty(config)
        .context("Failed to serialize config")?;
    fs::write(path, json)
        .context("Failed to write config file")?;
    Ok(())
}

