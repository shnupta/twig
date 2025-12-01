use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

pub struct DataPaths {
    base_dir: PathBuf,
}

impl DataPaths {
    pub fn new() -> Result<Self> {
        let home = std::env::var("HOME")
            .context("HOME environment variable not set")?;
        let base_dir = Path::new(&home).join(".twig");
        
        // Create directories if they don't exist
        std::fs::create_dir_all(&base_dir)
            .context("Failed to create .twig directory")?;
        std::fs::create_dir_all(base_dir.join("reportees"))
            .context("Failed to create reportees directory")?;
        
        Ok(Self { base_dir })
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn tasks_file(&self) -> PathBuf {
        self.base_dir.join("tasks.json")
    }

    pub fn config_file(&self) -> PathBuf {
        self.base_dir.join("config.json")
    }

    pub fn reportee_tasks_file(&self, name: &str) -> PathBuf {
        self.base_dir.join("reportees").join(format!("{}.json", name))
    }

    pub fn reportees_dir(&self) -> PathBuf {
        self.base_dir.join("reportees")
    }

    pub fn list_reportee_files(&self) -> Result<Vec<String>> {
        let dir = self.reportees_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut names = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    names.push(stem.to_string());
                }
            }
        }
        Ok(names)
    }
}

impl Default for DataPaths {
    fn default() -> Self {
        Self::new().expect("Failed to initialize data paths")
    }
}

