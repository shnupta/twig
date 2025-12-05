use crate::models::{Task, TaskStatus};
use crate::storage::{DataPaths, Storage};
use crate::tui::ui;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

pub enum AppMode {
    Normal,
    Help,
    AddTask,
    EditTask,
    DeleteConfirm,
}

pub struct InputState {
    pub title: String,
    pub description: String,
    pub tags: String,
    pub estimate: String,
    pub note: String,
    pub current_field: usize,
}

pub enum ViewTab {
    MyTasks,
    AllReportees,
    History,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HistoryPeriod {
    Day,
    Week,
    Month,
}

#[derive(Debug, Clone)]
pub enum VisibleItem {
    ReporteeHeader(String), // reportee name
    Task { id: uuid::Uuid, owner: String },
}

pub enum VisibleItemInfo<'a> {
    ReporteeHeader {
        name: &'a str,
        is_expanded: bool,
    },
    Task {
        task: &'a Task,
        depth: usize,
        owner: &'a str,
    },
}

pub struct App {
    pub storage: Storage,
    pub selected_index: usize,
    pub mode: AppMode,
    pub view_tab: ViewTab,
    pub reportees: Vec<String>,
    pub reportee_storages: std::collections::HashMap<String, Storage>,
    pub show_completed: bool,
    pub show_cancelled: bool,
    pub filter_tag: Option<String>,
    pub expanded_tasks: Vec<uuid::Uuid>,
    pub expanded_reportees: Vec<String>, // which reportee sections are expanded
    pub should_quit: bool,
    pub input_state: InputState,
    pub editing_task_id: Option<uuid::Uuid>,
    pub visible_task_list: Vec<VisibleItem>,
    // History view state
    pub history_period: HistoryPeriod,
    pub history_date: chrono::NaiveDate,
}

impl App {
    pub fn new() -> Result<Self> {
        let paths = DataPaths::new()?;
        let mut storage = Storage::new(paths.tasks_file().to_string_lossy().to_string());
        storage.load()?;

        // Load reportees
        let config = crate::storage::json_store::load_config(&paths.config_file())?;
        let reportees = config.reportees.clone();

        // Load reportee storages
        let mut reportee_storages = std::collections::HashMap::new();
        for reportee in &reportees {
            let reportee_path = paths.reportee_tasks_file(reportee);
            let mut reportee_storage = Storage::new(reportee_path.to_string_lossy().to_string());
            let _ = reportee_storage.load(); // Ignore errors for now
            reportee_storages.insert(reportee.clone(), reportee_storage);
        }

        Ok(Self {
            storage,
            selected_index: 0,
            mode: AppMode::Normal,
            view_tab: ViewTab::MyTasks,
            reportees,
            reportee_storages,
            show_completed: true,
            show_cancelled: false,
            filter_tag: None,
            expanded_tasks: Vec::new(),
            expanded_reportees: Vec::new(),
            should_quit: false,
            input_state: InputState {
                title: String::new(),
                description: String::new(),
                tags: String::new(),
                estimate: String::new(),
                note: String::new(),
                current_field: 0,
            },
            editing_task_id: None,
            visible_task_list: Vec::new(),
            history_period: HistoryPeriod::Day,
            history_date: chrono::Local::now().date_naive(),
        })
    }

    pub fn switch_tab(&mut self) {
        self.view_tab = match &self.view_tab {
            ViewTab::MyTasks => {
                if self.reportees.is_empty() {
                    ViewTab::History
                } else {
                    ViewTab::AllReportees
                }
            }
            ViewTab::AllReportees => ViewTab::History,
            ViewTab::History => ViewTab::MyTasks,
        };
        self.selected_index = 0;
        self.rebuild_visible_task_list();
    }

    pub fn switch_to_tab(&mut self, tab_num: usize) {
        self.view_tab = match tab_num {
            1 => ViewTab::MyTasks,
            2 if !self.reportees.is_empty() => ViewTab::AllReportees,
            3 => ViewTab::History,
            _ => return,
        };
        self.selected_index = 0;
        self.rebuild_visible_task_list();
    }

    pub fn rebuild_visible_task_list(&mut self) {
        self.visible_task_list.clear();

        match &self.view_tab {
            ViewTab::MyTasks => {
                let root_task_ids: Vec<uuid::Uuid> = self
                    .storage
                    .get_root_tasks()
                    .into_iter()
                    .filter(|t| self.should_show_task(t))
                    .map(|t| t.id)
                    .collect();

                for root_id in root_task_ids {
                    self.add_task_to_visible_list(root_id, "me".to_string());
                }
            }
            ViewTab::AllReportees => {
                let reportees = self.reportees.clone();
                for reportee in &reportees {
                    // Always add reportee header
                    self.visible_task_list
                        .push(VisibleItem::ReporteeHeader(reportee.clone()));

                    // If reportee is expanded, show their tasks
                    if self.expanded_reportees.contains(reportee) {
                        let root_task_ids: Vec<uuid::Uuid> =
                            if let Some(storage) = self.reportee_storages.get(reportee) {
                                storage
                                    .get_root_tasks()
                                    .into_iter()
                                    .filter(|t| self.should_show_task(t))
                                    .map(|t| t.id)
                                    .collect()
                            } else {
                                vec![]
                            };

                        for root_id in root_task_ids {
                            self.add_task_to_visible_list(root_id, reportee.clone());
                        }
                    }
                }
            }
            ViewTab::History => {
                // Show tasks completed/cancelled in the selected period
                self.rebuild_history_list();
            }
        }
    }

    fn rebuild_history_list(&mut self) {
        use chrono::{Datelike, Duration};

        let (start_date, end_date) = match self.history_period {
            HistoryPeriod::Day => (self.history_date, self.history_date),
            HistoryPeriod::Week => {
                // Start of week (Monday) to end of week (Sunday)
                let days_from_monday = self.history_date.weekday().num_days_from_monday();
                let start = self.history_date - Duration::days(days_from_monday as i64);
                let end = start + Duration::days(6);
                (start, end)
            }
            HistoryPeriod::Month => {
                // Start of month to end of month
                let start = self.history_date.with_day(1).unwrap();
                let next_month = if self.history_date.month() == 12 {
                    chrono::NaiveDate::from_ymd_opt(self.history_date.year() + 1, 1, 1).unwrap()
                } else {
                    chrono::NaiveDate::from_ymd_opt(
                        self.history_date.year(),
                        self.history_date.month() + 1,
                        1,
                    )
                    .unwrap()
                };
                let end = next_month - Duration::days(1);
                (start, end)
            }
        };

        // Collect all completed/cancelled tasks from my storage
        let my_history: Vec<uuid::Uuid> = self
            .storage
            .get_all_tasks()
            .iter()
            .filter(|t| self.is_task_in_history_range(t, start_date, end_date))
            .map(|t| t.id)
            .collect();

        for task_id in my_history {
            self.visible_task_list.push(VisibleItem::Task {
                id: task_id,
                owner: "me".to_string(),
            });
        }

        // Also include reportee tasks
        let reportees = self.reportees.clone();
        for reportee in &reportees {
            if let Some(storage) = self.reportee_storages.get(reportee) {
                let reportee_history: Vec<uuid::Uuid> = storage
                    .get_all_tasks()
                    .iter()
                    .filter(|t| self.is_task_in_history_range(t, start_date, end_date))
                    .map(|t| t.id)
                    .collect();

                for task_id in reportee_history {
                    self.visible_task_list.push(VisibleItem::Task {
                        id: task_id,
                        owner: reportee.clone(),
                    });
                }
            }
        }
    }

    fn is_task_in_history_range(
        &self,
        task: &Task,
        start: chrono::NaiveDate,
        end: chrono::NaiveDate,
    ) -> bool {
        use chrono::Local;

        // Check if task was completed in range
        if let Some(completed_at) = task.completed_at {
            let completed_date = completed_at.with_timezone(&Local).date_naive();
            if completed_date >= start && completed_date <= end {
                return true;
            }
        }

        // Check if task was cancelled in range
        if let Some(cancelled_at) = task.cancelled_at {
            let cancelled_date = cancelled_at.with_timezone(&Local).date_naive();
            if cancelled_date >= start && cancelled_date <= end {
                return true;
            }
        }

        false
    }

    fn add_task_to_visible_list(&mut self, task_id: uuid::Uuid, owner: String) {
        self.visible_task_list.push(VisibleItem::Task {
            id: task_id,
            owner: owner.clone(),
        });

        // If task is expanded, add its children
        if self.expanded_tasks.contains(&task_id) {
            let storage = self.get_storage_for_owner(&owner);
            let child_ids: Vec<uuid::Uuid> = storage
                .get_children(task_id)
                .into_iter()
                .filter(|c| self.should_show_task(c))
                .map(|c| c.id)
                .collect();

            for child_id in child_ids {
                self.add_task_to_visible_list(child_id, owner.clone());
            }
        }
    }

    fn should_show_task(&self, task: &Task) -> bool {
        use chrono::Local;

        let today = Local::now().date_naive();

        // For completed tasks: show if completed today, or if show_completed is enabled
        if task.status == TaskStatus::Completed {
            if let Some(completed_at) = task.completed_at {
                let completed_date = completed_at.with_timezone(&Local).date_naive();
                if completed_date == today {
                    // Always show tasks completed today
                } else if !self.show_completed {
                    // Hide tasks completed on previous days unless filter is on
                    return false;
                }
            } else if !self.show_completed {
                return false;
            }
        }

        // For cancelled tasks: show if cancelled today, or if show_cancelled is enabled
        if task.status == TaskStatus::Cancelled {
            if let Some(cancelled_at) = task.cancelled_at {
                let cancelled_date = cancelled_at.with_timezone(&Local).date_naive();
                if cancelled_date == today {
                    // Always show tasks cancelled today
                } else if !self.show_cancelled {
                    // Hide tasks cancelled on previous days unless filter is on
                    return false;
                }
            } else if !self.show_cancelled {
                return false;
            }
        }

        if let Some(ref tag) = self.filter_tag {
            if !task.tags.contains(tag) {
                return false;
            }
        }
        true
    }

    pub fn get_visible_items(&self) -> Vec<VisibleItemInfo> {
        let mut result = Vec::new();
        for item in &self.visible_task_list {
            match item {
                VisibleItem::ReporteeHeader(name) => {
                    result.push(VisibleItemInfo::ReporteeHeader {
                        name: name.as_str(),
                        is_expanded: self.expanded_reportees.contains(name),
                    });
                }
                VisibleItem::Task { id, owner } => {
                    let storage = self.get_storage_for_owner(owner);
                    if let Some(task) = storage.get_task(*id) {
                        let depth = self.get_task_depth(task, storage);
                        result.push(VisibleItemInfo::Task {
                            task,
                            depth,
                            owner: owner.as_str(),
                        });
                    }
                }
            }
        }
        result
    }

    fn get_task_depth(&self, task: &Task, storage: &Storage) -> usize {
        let mut depth = 0;
        let mut current_id = task.parent_id;
        while let Some(id) = current_id {
            depth += 1;
            if let Some(parent) = storage.get_task(id) {
                current_id = parent.parent_id;
            } else {
                break;
            }
        }
        depth
    }

    pub fn get_selected_item(&self) -> Option<&VisibleItem> {
        if self.selected_index < self.visible_task_list.len() {
            Some(&self.visible_task_list[self.selected_index])
        } else {
            None
        }
    }

    pub fn get_selected_task(&self) -> Option<(&Task, &str)> {
        match self.get_selected_item()? {
            VisibleItem::Task { id, owner } => {
                let storage = self.get_storage_for_owner(owner);
                storage.get_task(*id).map(|t| (t, owner.as_str()))
            }
            VisibleItem::ReporteeHeader(_) => None,
        }
    }

    pub fn get_storage_for_owner(&self, owner: &str) -> &Storage {
        if owner == "me" {
            &self.storage
        } else {
            self.reportee_storages.get(owner).unwrap_or(&self.storage)
        }
    }

    fn get_storage_for_owner_mut(&mut self, owner: &str) -> &mut Storage {
        if owner == "me" {
            &mut self.storage
        } else {
            self.reportee_storages.get_mut(owner).unwrap()
        }
    }

    pub fn has_children(&self, task_id: uuid::Uuid, owner: &str) -> bool {
        let storage = self.get_storage_for_owner(owner);
        !storage.get_children(task_id).is_empty()
    }

    pub fn is_expanded(&self, task_id: uuid::Uuid) -> bool {
        self.expanded_tasks.contains(&task_id)
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_selection_down(&mut self) {
        if self.selected_index < self.visible_task_list.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    pub fn toggle_expand(&mut self) {
        match self.get_selected_item() {
            Some(VisibleItem::Task { id, owner }) => {
                if !self.has_children(*id, owner) {
                    return; // No children to expand
                }

                if let Some(pos) = self.expanded_tasks.iter().position(|&x| x == *id) {
                    self.expanded_tasks.remove(pos);
                } else {
                    self.expanded_tasks.push(*id);
                }
                self.rebuild_visible_task_list();
            }
            Some(VisibleItem::ReporteeHeader(name)) => {
                // Toggle reportee expansion
                if let Some(pos) = self.expanded_reportees.iter().position(|n| n == name) {
                    self.expanded_reportees.remove(pos);
                } else {
                    self.expanded_reportees.push(name.clone());
                }
                self.rebuild_visible_task_list();
            }
            None => {}
        }
    }

    pub fn toggle_completed(&mut self) {
        self.show_completed = !self.show_completed;
        self.rebuild_visible_task_list();
    }

    pub fn toggle_cancelled(&mut self) {
        self.show_cancelled = !self.show_cancelled;
        self.rebuild_visible_task_list();
    }

    pub fn start_add_task(&mut self, as_subtask: bool) {
        self.input_state = InputState {
            title: String::new(),
            description: String::new(),
            tags: String::new(),
            estimate: String::new(),
            note: String::new(),
            current_field: 0,
        };
        // Store whether this should be a subtask or top-level
        // For reportee headers, this should be None (top-level for that reportee)
        self.editing_task_id = if as_subtask {
            // Only if we're on an actual task
            self.get_selected_task().map(|(t, _)| t.id)
        } else {
            None
        };
        self.mode = AppMode::AddTask;
    }

    pub fn start_edit_task(&mut self) {
        if let Some((task, _owner)) = self.get_selected_task() {
            let task_id = task.id;
            let title = task.title.clone();
            let description = task.description.clone();
            let tags = task.tags.join(", ");
            let estimate = task.get_formatted_estimate().unwrap_or_default();
            let notes = task.notes.clone();

            self.editing_task_id = Some(task_id);
            self.input_state = InputState {
                title,
                description,
                tags,
                estimate,
                note: notes,
                current_field: 0,
            };
            self.mode = AppMode::EditTask;
        }
    }

    pub fn save_new_task(&mut self) -> Result<()> {
        let mut task = Task::new(self.input_state.title.clone());
        task.description = self.input_state.description.clone();

        if !self.input_state.tags.is_empty() {
            task.tags = self
                .input_state
                .tags
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }

        if !self.input_state.estimate.is_empty() {
            let _ = task.set_estimate(&self.input_state.estimate);
        }

        task.notes = self.input_state.note.clone();

        // Set parent based on editing_task_id (which stores the parent for new tasks)
        if let Some(parent_id) = self.editing_task_id {
            task.parent_id = Some(parent_id);
        }

        // Determine which storage to add to
        let owner: String = match &self.view_tab {
            ViewTab::MyTasks => "me".to_string(),
            ViewTab::AllReportees => {
                // If adding as subtask, use parent's owner
                if let Some(parent_id) = self.editing_task_id {
                    self.visible_task_list
                        .iter()
                        .find_map(|item| {
                            if let VisibleItem::Task { id, owner } = item {
                                if *id == parent_id {
                                    return Some(owner.clone());
                                }
                            }
                            None
                        })
                        .unwrap_or_else(|| "me".to_string())
                } else {
                    // Check if we're on a reportee header
                    if let Some(VisibleItem::ReporteeHeader(name)) = self.get_selected_item() {
                        name.clone()
                    } else {
                        "me".to_string()
                    }
                }
            }
            ViewTab::History => "me".to_string(), // History view doesn't allow adding tasks
        };

        let storage = self.get_storage_for_owner_mut(&owner);
        storage.add_task(task)?;
        self.rebuild_visible_task_list();
        self.editing_task_id = None;
        self.mode = AppMode::Normal;
        Ok(())
    }

    pub fn save_edit_task(&mut self) -> Result<()> {
        if let Some(task_id) = self.editing_task_id {
            // Clone all the input data first
            let title = self.input_state.title.clone();
            let description = self.input_state.description.clone();
            let tags = self.input_state.tags.clone();
            let estimate = self.input_state.estimate.clone();
            let notes = self.input_state.note.clone();

            // Get the owner from visible list
            let owner = self
                .visible_task_list
                .iter()
                .find_map(|item| {
                    if let VisibleItem::Task { id, owner } = item {
                        if *id == task_id {
                            return Some(owner.clone());
                        }
                    }
                    None
                })
                .unwrap_or_else(|| "me".to_string());

            {
                let storage = self.get_storage_for_owner_mut(&owner);
                if let Some(task) = storage.get_task_mut(task_id) {
                    task.title = title;
                    task.description = description;

                    if !tags.is_empty() {
                        task.tags = tags.split(',').map(|s| s.trim().to_string()).collect();
                    } else {
                        task.tags.clear();
                    }

                    if !estimate.is_empty() {
                        let _ = task.set_estimate(&estimate);
                    } else {
                        task.estimated_effort_hours = None;
                    }

                    task.notes = notes;
                }
                storage.save()?;
            }
        }
        self.editing_task_id = None;
        self.mode = AppMode::Normal;
        Ok(())
    }

    pub fn cancel_input(&mut self) {
        self.editing_task_id = None;
        self.mode = AppMode::Normal;
    }

    pub fn start_delete_task(&mut self) {
        if let Some((task, _owner)) = self.get_selected_task() {
            self.editing_task_id = Some(task.id);
            self.mode = AppMode::DeleteConfirm;
        }
    }

    pub fn confirm_delete_task(&mut self) -> Result<()> {
        if let Some(task_id) = self.editing_task_id {
            // Find owner
            let owner = self
                .visible_task_list
                .iter()
                .find_map(|item| {
                    if let VisibleItem::Task { id, owner } = item {
                        if *id == task_id {
                            return Some(owner.clone());
                        }
                    }
                    None
                })
                .unwrap_or_else(|| "me".to_string());

            self.get_storage_for_owner_mut(&owner)
                .delete_task(task_id)?;
            self.rebuild_visible_task_list();
            // Adjust selection if needed
            if self.selected_index >= self.visible_task_list.len() && self.selected_index > 0 {
                self.selected_index -= 1;
            }
        }
        self.editing_task_id = None;
        self.mode = AppMode::Normal;
        Ok(())
    }

    pub fn get_task_by_id_with_owner(&self, id: uuid::Uuid) -> Option<(&Task, &str)> {
        // Try to find in visible list first
        for item in &self.visible_task_list {
            if let VisibleItem::Task { id: task_id, owner } = item {
                if *task_id == id {
                    let storage = self.get_storage_for_owner(owner);
                    return storage.get_task(id).map(|t| (t, owner.as_str()));
                }
            }
        }

        // Fallback: search all storages
        if let Some(task) = self.storage.get_task(id) {
            return Some((task, "me"));
        }

        for (name, storage) in &self.reportee_storages {
            if let Some(task) = storage.get_task(id) {
                return Some((task, name.as_str()));
            }
        }

        None
    }

    pub fn start_selected_task(&mut self) -> Result<()> {
        if let Some((task, owner)) = self.get_selected_task() {
            let task_id = task.id;
            let owner = owner.to_string();
            {
                let storage = self.get_storage_for_owner_mut(&owner);
                if let Some(task_mut) = storage.get_task_mut(task_id) {
                    task_mut.start();
                }
                storage.save()?;
            }
        }
        Ok(())
    }

    pub fn complete_selected_task(&mut self) -> Result<()> {
        if let Some((task, owner)) = self.get_selected_task() {
            let task_id = task.id;
            let owner = owner.to_string();
            {
                let storage = self.get_storage_for_owner_mut(&owner);
                if let Some(task_mut) = storage.get_task_mut(task_id) {
                    task_mut.complete();
                }
                storage.save()?;
            }
        }
        Ok(())
    }

    pub fn cancel_selected_task(&mut self) -> Result<()> {
        if let Some((task, owner)) = self.get_selected_task() {
            let task_id = task.id;
            let owner = owner.to_string();
            {
                let storage = self.get_storage_for_owner_mut(&owner);
                if let Some(task_mut) = storage.get_task_mut(task_id) {
                    task_mut.cancel();
                }
                storage.save()?;
            }
        }
        Ok(())
    }

    pub fn pause_selected_task(&mut self) -> Result<()> {
        if let Some((task, owner)) = self.get_selected_task() {
            let task_id = task.id;
            let owner = owner.to_string();
            {
                let storage = self.get_storage_for_owner_mut(&owner);
                if let Some(task_mut) = storage.get_task_mut(task_id) {
                    if task_mut.has_active_time_entry() {
                        task_mut.pause();
                    }
                }
                storage.save()?;
            }
        }
        Ok(())
    }

    pub fn reload(&mut self) -> Result<()> {
        self.storage.load()?;
        for storage in self.reportee_storages.values_mut() {
            let _ = storage.load();
        }
        self.rebuild_visible_task_list();
        Ok(())
    }

    pub fn input_char(&mut self, c: char) {
        let field = match self.input_state.current_field {
            0 => &mut self.input_state.title,
            1 => &mut self.input_state.description,
            2 => &mut self.input_state.tags,
            3 => &mut self.input_state.estimate,
            4 => &mut self.input_state.note,
            _ => return,
        };
        field.push(c);
    }

    pub fn input_backspace(&mut self) {
        let field = match self.input_state.current_field {
            0 => &mut self.input_state.title,
            1 => &mut self.input_state.description,
            2 => &mut self.input_state.tags,
            3 => &mut self.input_state.estimate,
            4 => &mut self.input_state.note,
            _ => return,
        };
        field.pop();
    }

    pub fn next_field(&mut self) {
        // Fields: 0=title, 1=description, 2=tags, 3=estimate, 4=note, 5=Save, 6=Cancel
        self.input_state.current_field = (self.input_state.current_field + 1).min(6);
    }

    pub fn prev_field(&mut self) {
        self.input_state.current_field = self.input_state.current_field.saturating_sub(1);
    }

    // History navigation
    pub fn history_next_period(&mut self) {
        use chrono::{Datelike, Duration};

        self.history_date = match self.history_period {
            HistoryPeriod::Day => self.history_date + Duration::days(1),
            HistoryPeriod::Week => self.history_date + Duration::weeks(1),
            HistoryPeriod::Month => {
                if self.history_date.month() == 12 {
                    chrono::NaiveDate::from_ymd_opt(
                        self.history_date.year() + 1,
                        1,
                        self.history_date.day().min(28),
                    )
                    .unwrap()
                } else {
                    let next_month = self.history_date.month() + 1;
                    let day = self.history_date.day().min(28); // Safe day for all months
                    chrono::NaiveDate::from_ymd_opt(self.history_date.year(), next_month, day)
                        .unwrap()
                }
            }
        };
        self.rebuild_visible_task_list();
    }

    pub fn history_prev_period(&mut self) {
        use chrono::{Datelike, Duration};

        self.history_date = match self.history_period {
            HistoryPeriod::Day => self.history_date - Duration::days(1),
            HistoryPeriod::Week => self.history_date - Duration::weeks(1),
            HistoryPeriod::Month => {
                if self.history_date.month() == 1 {
                    chrono::NaiveDate::from_ymd_opt(
                        self.history_date.year() - 1,
                        12,
                        self.history_date.day().min(28),
                    )
                    .unwrap()
                } else {
                    let prev_month = self.history_date.month() - 1;
                    let day = self.history_date.day().min(28);
                    chrono::NaiveDate::from_ymd_opt(self.history_date.year(), prev_month, day)
                        .unwrap()
                }
            }
        };
        self.rebuild_visible_task_list();
    }

    pub fn history_cycle_period(&mut self) {
        self.history_period = match self.history_period {
            HistoryPeriod::Day => HistoryPeriod::Week,
            HistoryPeriod::Week => HistoryPeriod::Month,
            HistoryPeriod::Month => HistoryPeriod::Day,
        };
        self.rebuild_visible_task_list();
    }

    pub fn history_goto_today(&mut self) {
        self.history_date = chrono::Local::now().date_naive();
        self.rebuild_visible_task_list();
    }

    pub fn get_history_period_label(&self) -> String {
        use chrono::Datelike;

        match self.history_period {
            HistoryPeriod::Day => self.history_date.format("%A, %B %d, %Y").to_string(),
            HistoryPeriod::Week => {
                let days_from_monday = self.history_date.weekday().num_days_from_monday();
                let start = self.history_date - chrono::Duration::days(days_from_monday as i64);
                let end = start + chrono::Duration::days(6);
                format!(
                    "Week of {} - {}",
                    start.format("%b %d"),
                    end.format("%b %d, %Y")
                )
            }
            HistoryPeriod::Month => self.history_date.format("%B %Y").to_string(),
        }
    }
}

pub fn run_tui() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new()?;
    app.rebuild_visible_task_list();

    // Run event loop
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if let Event::Key(key) = event::read()? {
            match app.mode {
                AppMode::Normal => {
                    match key.code {
                        KeyCode::Char('q') => {
                            app.should_quit = true;
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.should_quit = true;
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.move_selection_down();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.move_selection_up();
                        }
                        KeyCode::Char('s') => {
                            app.start_selected_task()?;
                        }
                        KeyCode::Char('c') => {
                            app.complete_selected_task()?;
                        }
                        KeyCode::Char('x') => {
                            app.cancel_selected_task()?;
                        }
                        KeyCode::Char('p') => {
                            app.pause_selected_task()?;
                        }
                        KeyCode::Char('h') => {
                            app.toggle_completed();
                        }
                        KeyCode::Char('H') => {
                            app.toggle_cancelled();
                        }
                        KeyCode::Char('r') => {
                            app.reload()?;
                        }
                        KeyCode::Char('?') => {
                            app.mode = AppMode::Help;
                        }
                        KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Tab => {
                            app.toggle_expand();
                        }
                        KeyCode::Char('a') => {
                            if !matches!(app.view_tab, ViewTab::History) {
                                app.start_add_task(true); // Add as subtask
                            }
                        }
                        KeyCode::Char('A') => {
                            if !matches!(app.view_tab, ViewTab::History) {
                                app.start_add_task(false); // Add as top-level task
                            }
                        }
                        KeyCode::Char('e') => {
                            if !matches!(app.view_tab, ViewTab::History) {
                                app.start_edit_task();
                            }
                        }
                        KeyCode::Char('d') => {
                            if !matches!(app.view_tab, ViewTab::History) {
                                app.start_delete_task();
                            }
                        }
                        KeyCode::Char('m') => {
                            if matches!(app.view_tab, ViewTab::History) {
                                app.history_cycle_period();
                            }
                        }
                        KeyCode::Char('t') => {
                            if matches!(app.view_tab, ViewTab::History) {
                                app.history_goto_today();
                            }
                        }
                        KeyCode::Right => {
                            if matches!(app.view_tab, ViewTab::History) {
                                app.history_next_period();
                            } else {
                                app.switch_tab();
                            }
                        }
                        KeyCode::Left => {
                            if matches!(app.view_tab, ViewTab::History) {
                                app.history_prev_period();
                            } else {
                                app.switch_tab();
                            }
                        }
                        KeyCode::Char('1') => {
                            app.switch_to_tab(1);
                        }
                        KeyCode::Char('2') => {
                            if app.reportees.is_empty() {
                                app.switch_to_tab(3); // Go to History if no reportees
                            } else {
                                app.switch_to_tab(2);
                            }
                        }
                        KeyCode::Char('3') => {
                            app.switch_to_tab(3);
                        }
                        _ => {}
                    }
                }
                AppMode::Help => {
                    if matches!(
                        key.code,
                        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?')
                    ) {
                        app.mode = AppMode::Normal;
                    }
                }
                AppMode::DeleteConfirm => match key.code {
                    KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                        let _ = app.confirm_delete_task();
                    }
                    KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                        app.cancel_input();
                    }
                    _ => {}
                },
                AppMode::AddTask | AppMode::EditTask => {
                    match key.code {
                        KeyCode::Esc => {
                            app.cancel_input();
                        }
                        KeyCode::Char(c) => {
                            app.input_char(c);
                        }
                        KeyCode::Backspace => {
                            app.input_backspace();
                        }
                        KeyCode::Tab => {
                            app.next_field();
                        }
                        KeyCode::BackTab => {
                            app.prev_field();
                        }
                        KeyCode::Down => {
                            app.next_field();
                        }
                        KeyCode::Up => {
                            app.prev_field();
                        }
                        KeyCode::Enter => {
                            // Check for Ctrl+Enter first (save from any field)
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                if matches!(app.mode, AppMode::AddTask) {
                                    let _ = app.save_new_task();
                                } else {
                                    let _ = app.save_edit_task();
                                }
                            } else if app.input_state.current_field == 5 {
                                // Save button selected
                                if matches!(app.mode, AppMode::AddTask) {
                                    let _ = app.save_new_task();
                                } else {
                                    let _ = app.save_edit_task();
                                }
                            } else if app.input_state.current_field == 6 {
                                // Cancel button selected
                                app.cancel_input();
                            } else if app.input_state.current_field == 4 {
                                // Regular Enter in note field inserts newline
                                app.input_char('\n');
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
