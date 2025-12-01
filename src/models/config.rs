use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub reportees: Vec<String>,
    pub default_view: ViewMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    Tree,
    List,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            reportees: Vec::new(),
            default_view: ViewMode::Tree,
        }
    }
}

impl Config {
    pub fn add_reportee(&mut self, name: String) -> bool {
        if !self.reportees.contains(&name) {
            self.reportees.push(name);
            true
        } else {
            false
        }
    }

    pub fn remove_reportee(&mut self, name: &str) -> bool {
        if let Some(pos) = self.reportees.iter().position(|r| r == name) {
            self.reportees.remove(pos);
            true
        } else {
            false
        }
    }
}
