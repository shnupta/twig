use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub struct DataPaths {
    base_dir: PathBuf,
}

impl DataPaths {
    pub fn new() -> Result<Self> {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        let base_dir = Path::new(&home).join(".twig");

        // Create directories if they don't exist
        std::fs::create_dir_all(&base_dir).context("Failed to create .twig directory")?;
        std::fs::create_dir_all(base_dir.join("reportees"))
            .context("Failed to create reportees directory")?;

        Ok(Self { base_dir })
    }

    pub fn tasks_file(&self) -> PathBuf {
        self.base_dir.join("tasks.json")
    }

    pub fn config_file(&self) -> PathBuf {
        self.base_dir.join("config.json")
    }

    pub fn reportee_tasks_file(&self, name: &str) -> PathBuf {
        self.base_dir
            .join("reportees")
            .join(format!("{}.json", name))
    }
}

impl Default for DataPaths {
    fn default() -> Self {
        Self::new().expect("Failed to initialize data paths")
    }
}
