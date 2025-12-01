use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    NotStarted,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEntry {
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i64>,
}

impl TimeEntry {
    pub fn new(start: DateTime<Utc>) -> Self {
        Self {
            start,
            end: None,
            duration_seconds: None,
        }
    }

    pub fn end_entry(&mut self, end: DateTime<Utc>) {
        self.end = Some(end);
        self.duration_seconds = Some((end - self.start).num_seconds());
    }

    pub fn is_active(&self) -> bool {
        self.end.is_none()
    }
}

#[derive(Debug, Clone)]
pub struct EffortEstimate {
    pub value: f64,
    pub unit: EffortUnit,
}

#[derive(Debug, Clone, Copy)]
pub enum EffortUnit {
    Hours,
    Days,
    Weeks,
    Months,
}

impl EffortEstimate {
    /// Parse effort string like "1h", "2d", "3w", "2m"
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        let s = s.trim().to_lowercase();
        let (num_str, unit_str) = s.split_at(s.len() - 1);

        let value: f64 = num_str
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid effort value: {}", num_str))?;

        let unit = match unit_str {
            "h" => EffortUnit::Hours,
            "d" => EffortUnit::Days,
            "w" => EffortUnit::Weeks,
            "m" => EffortUnit::Months,
            _ => {
                return Err(anyhow::anyhow!(
                    "Invalid effort unit: {}. Use h/d/w/m",
                    unit_str
                ))
            }
        };

        Ok(Self { value, unit })
    }

    /// Convert to hours for storage
    pub fn to_hours(&self) -> f64 {
        match self.unit {
            EffortUnit::Hours => self.value,
            EffortUnit::Days => self.value * 8.0, // 8 hour workday
            EffortUnit::Weeks => self.value * 40.0, // 5 day work week
            EffortUnit::Months => self.value * 160.0, // ~4 weeks per month
        }
    }

    /// Format hours back to human readable format
    pub fn from_hours(hours: f64) -> String {
        if hours < 8.0 {
            format!("{:.1}h", hours)
        } else if hours < 40.0 {
            format!("{:.1}d", hours / 8.0)
        } else if hours < 160.0 {
            format!("{:.1}w", hours / 40.0)
        } else {
            format!("{:.1}m", hours / 160.0)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub parent_id: Option<Uuid>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub estimated_effort_hours: Option<f64>,
    pub eta: Option<DateTime<Utc>>,
    pub time_entries: Vec<TimeEntry>,
    pub total_time_seconds: i64,
    #[serde(default)]
    pub notes: String,
}

impl Task {
    pub fn new(title: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            title,
            description: String::new(),
            status: TaskStatus::NotStarted,
            parent_id: None,
            tags: Vec::new(),
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            cancelled_at: None,
            estimated_effort_hours: None,
            eta: None,
            time_entries: Vec::new(),
            total_time_seconds: 0,
            notes: String::new(),
        }
    }

    pub fn start(&mut self) {
        if self.status == TaskStatus::NotStarted {
            self.started_at = Some(Utc::now());
        }
        self.status = TaskStatus::InProgress;

        // Start a new time entry
        self.time_entries.push(TimeEntry::new(Utc::now()));
    }

    pub fn complete(&mut self) {
        self.status = TaskStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.end_active_time_entry();
    }

    pub fn cancel(&mut self) {
        self.status = TaskStatus::Cancelled;
        self.cancelled_at = Some(Utc::now());
        self.end_active_time_entry();
    }

    pub fn pause(&mut self) {
        self.end_active_time_entry();
    }

    fn end_active_time_entry(&mut self) {
        if let Some(entry) = self.time_entries.iter_mut().find(|e| e.is_active()) {
            let now = Utc::now();
            entry.end_entry(now);
            self.total_time_seconds += entry.duration_seconds.unwrap_or(0);
        }
    }

    pub fn has_active_time_entry(&self) -> bool {
        self.time_entries.iter().any(|e| e.is_active())
    }

    pub fn set_estimate(&mut self, estimate: &str) -> anyhow::Result<()> {
        let effort = EffortEstimate::parse(estimate)?;
        self.estimated_effort_hours = Some(effort.to_hours());
        Ok(())
    }

    pub fn get_formatted_estimate(&self) -> Option<String> {
        self.estimated_effort_hours.map(EffortEstimate::from_hours)
    }

    pub fn get_formatted_total_time(&self) -> String {
        let seconds = self.total_time_seconds;
        if seconds < 60 {
            return format!("{}s", seconds);
        }

        let minutes = seconds / 60;
        let hours = minutes / 60;
        let days = hours / 8; // 8-hour work days

        let remaining_hours = hours % 8;
        let remaining_minutes = minutes % 60;

        let mut parts = Vec::new();

        if days > 0 {
            parts.push(format!("{}d", days));
        }
        if remaining_hours > 0 || days > 0 {
            parts.push(format!("{}h", remaining_hours));
        }
        if remaining_minutes > 0 || parts.is_empty() {
            parts.push(format!("{}m", remaining_minutes));
        }

        parts.join(" ")
    }

    pub fn short_id(&self) -> String {
        self.id.to_string()[..8].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effort_parsing() {
        let cases = vec![
            ("1h", 1.0),
            ("2d", 16.0),
            ("1w", 40.0),
            ("2m", 320.0),
            ("0.5h", 0.5),
        ];

        for (input, expected_hours) in cases {
            let effort = EffortEstimate::parse(input).unwrap();
            assert_eq!(effort.to_hours(), expected_hours);
        }
    }

    #[test]
    fn test_task_lifecycle() {
        let mut task = Task::new("Test task".to_string());
        assert_eq!(task.status, TaskStatus::NotStarted);

        task.start();
        assert_eq!(task.status, TaskStatus::InProgress);
        assert!(task.started_at.is_some());
        assert!(task.has_active_time_entry());

        task.complete();
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.completed_at.is_some());
        assert!(!task.has_active_time_entry());
    }
}
