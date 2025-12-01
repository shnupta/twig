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
    Filter,
    Help,
    AddTask,
    EditTask,
}

pub struct InputState {
    pub title: String,
    pub description: String,
    pub tags: String,
    pub estimate: String,
    pub assignee: String,
    pub note: String,
    pub current_field: usize,
}

pub enum ViewTab {
    MyTasks,
    Reportee(String),
}

pub struct App {
    pub storage: Storage,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub mode: AppMode,
    pub view_tab: ViewTab,
    pub show_completed: bool,
    pub show_cancelled: bool,
    pub filter_tag: Option<String>,
    pub filter_assignee: Option<String>,
    pub expanded_tasks: Vec<uuid::Uuid>,
    pub should_quit: bool,
    pub input_state: InputState,
    pub editing_task_id: Option<uuid::Uuid>,
    pub visible_task_list: Vec<uuid::Uuid>, // Flat list of visible tasks in tree order
}

impl App {
    pub fn new() -> Result<Self> {
        let paths = DataPaths::new()?;
        let mut storage = Storage::new(paths.tasks_file().to_string_lossy().to_string());
        storage.load()?;

        Ok(Self {
            storage,
            selected_index: 0,
            scroll_offset: 0,
            mode: AppMode::Normal,
            view_tab: ViewTab::MyTasks,
            show_completed: true,
            show_cancelled: false,
            filter_tag: None,
            filter_assignee: None,
            expanded_tasks: Vec::new(),
            should_quit: false,
            input_state: InputState {
                title: String::new(),
                description: String::new(),
                tags: String::new(),
                estimate: String::new(),
                assignee: String::new(),
                note: String::new(),
                current_field: 0,
            },
            editing_task_id: None,
            visible_task_list: Vec::new(),
        })
    }

    pub fn rebuild_visible_task_list(&mut self) {
        self.visible_task_list.clear();
        let root_task_ids: Vec<uuid::Uuid> = self.storage.get_root_tasks()
            .into_iter()
            .filter(|t| self.should_show_task(t))
            .map(|t| t.id)
            .collect();
        
        for root_id in root_task_ids {
            self.add_task_to_visible_list(root_id, 0);
        }
    }

    fn add_task_to_visible_list(&mut self, task_id: uuid::Uuid, _depth: usize) {
        self.visible_task_list.push(task_id);
        
        // If task is expanded, add its children
        if self.expanded_tasks.contains(&task_id) {
            let child_ids: Vec<uuid::Uuid> = self.storage.get_children(task_id)
                .into_iter()
                .filter(|c| self.should_show_task(c))
                .map(|c| c.id)
                .collect();
                
            for child_id in child_ids {
                self.add_task_to_visible_list(child_id, _depth + 1);
            }
        }
    }

    fn should_show_task(&self, task: &Task) -> bool {
        if !self.show_completed && task.status == TaskStatus::Completed {
            return false;
        }
        if !self.show_cancelled && task.status == TaskStatus::Cancelled {
            return false;
        }
        if let Some(ref tag) = self.filter_tag {
            if !task.tags.contains(tag) {
                return false;
            }
        }
        if let Some(ref assignee) = self.filter_assignee {
            if task.assigned_to.as_deref() != Some(assignee) {
                return false;
            }
        }
        true
    }

    pub fn get_visible_tasks(&self) -> Vec<(&Task, usize)> {
        let mut result = Vec::new();
        for task_id in &self.visible_task_list {
            if let Some(task) = self.storage.get_task(*task_id) {
                let depth = self.get_task_depth(task);
                result.push((task, depth));
            }
        }
        result
    }

    fn get_task_depth(&self, task: &Task) -> usize {
        let mut depth = 0;
        let mut current_id = task.parent_id;
        while let Some(id) = current_id {
            depth += 1;
            if let Some(parent) = self.storage.get_task(id) {
                current_id = parent.parent_id;
            } else {
                break;
            }
        }
        depth
    }

    pub fn get_selected_task(&self) -> Option<&Task> {
        if self.selected_index < self.visible_task_list.len() {
            let task_id = self.visible_task_list[self.selected_index];
            self.storage.get_task(task_id)
        } else {
            None
        }
    }

    pub fn has_children(&self, task_id: uuid::Uuid) -> bool {
        !self.storage.get_children(task_id).is_empty()
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
        if let Some(task) = self.get_selected_task() {
            let id = task.id;
            if !self.has_children(id) {
                return; // No children to expand
            }
            
            if let Some(pos) = self.expanded_tasks.iter().position(|&x| x == id) {
                self.expanded_tasks.remove(pos);
            } else {
                self.expanded_tasks.push(id);
            }
            self.rebuild_visible_task_list();
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

    pub fn start_add_task(&mut self) {
        self.input_state = InputState {
            title: String::new(),
            description: String::new(),
            tags: String::new(),
            estimate: String::new(),
            assignee: String::new(),
            note: String::new(),
            current_field: 0,
        };
        self.mode = AppMode::AddTask;
    }

    pub fn start_edit_task(&mut self) {
        if let Some(task) = self.get_selected_task() {
            let task_id = task.id;
            let title = task.title.clone();
            let description = task.description.clone();
            let tags = task.tags.join(", ");
            let estimate = task.get_formatted_estimate().unwrap_or_default();
            let assignee = task.assigned_to.clone().unwrap_or_default();
            
            self.editing_task_id = Some(task_id);
            self.input_state = InputState {
                title,
                description,
                tags,
                estimate,
                assignee,
                note: String::new(),
                current_field: 0,
            };
            self.mode = AppMode::EditTask;
        }
    }

    pub fn save_new_task(&mut self) -> Result<()> {
        let mut task = Task::new(self.input_state.title.clone());
        task.description = self.input_state.description.clone();
        
        if !self.input_state.tags.is_empty() {
            task.tags = self.input_state.tags.split(',').map(|s| s.trim().to_string()).collect();
        }
        
        if !self.input_state.estimate.is_empty() {
            let _ = task.set_estimate(&self.input_state.estimate);
        }
        
        if !self.input_state.assignee.is_empty() {
            task.assigned_to = Some(self.input_state.assignee.clone());
        }
        
        if !self.input_state.note.is_empty() {
            task.add_note(self.input_state.note.clone());
        }
        
        // Set parent to selected task if one is selected
        if let Some(selected) = self.get_selected_task() {
            task.parent_id = Some(selected.id);
        }
        
        self.storage.add_task(task)?;
        self.rebuild_visible_task_list();
        self.mode = AppMode::Normal;
        Ok(())
    }

    pub fn save_edit_task(&mut self) -> Result<()> {
        if let Some(task_id) = self.editing_task_id {
            if let Some(task) = self.storage.get_task_mut(task_id) {
                task.title = self.input_state.title.clone();
                task.description = self.input_state.description.clone();
                
                if !self.input_state.tags.is_empty() {
                    task.tags = self.input_state.tags.split(',').map(|s| s.trim().to_string()).collect();
                } else {
                    task.tags.clear();
                }
                
                if !self.input_state.estimate.is_empty() {
                    let _ = task.set_estimate(&self.input_state.estimate);
                } else {
                    task.estimated_effort_hours = None;
                }
                
                if !self.input_state.assignee.is_empty() {
                    task.assigned_to = Some(self.input_state.assignee.clone());
                } else {
                    task.assigned_to = None;
                }
                
                if !self.input_state.note.is_empty() {
                    task.add_note(self.input_state.note.clone());
                }
                
                self.storage.save()?;
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

    pub fn start_selected_task(&mut self) -> Result<()> {
        if let Some(task) = self.get_selected_task() {
            let task_id = task.id;
            if let Some(task_mut) = self.storage.get_task_mut(task_id) {
                task_mut.start();
                self.storage.save()?;
            }
        }
        Ok(())
    }

    pub fn complete_selected_task(&mut self) -> Result<()> {
        if let Some(task) = self.get_selected_task() {
            let task_id = task.id;
            if let Some(task_mut) = self.storage.get_task_mut(task_id) {
                task_mut.complete();
                self.storage.save()?;
            }
        }
        Ok(())
    }

    pub fn cancel_selected_task(&mut self) -> Result<()> {
        if let Some(task) = self.get_selected_task() {
            let task_id = task.id;
            if let Some(task_mut) = self.storage.get_task_mut(task_id) {
                task_mut.cancel();
                self.storage.save()?;
            }
        }
        Ok(())
    }

    pub fn pause_selected_task(&mut self) -> Result<()> {
        if let Some(task) = self.get_selected_task() {
            let task_id = task.id;
            if let Some(task_mut) = self.storage.get_task_mut(task_id) {
                if task_mut.has_active_time_entry() {
                    task_mut.pause();
                    self.storage.save()?;
                }
            }
        }
        Ok(())
    }

    pub fn reload(&mut self) -> Result<()> {
        self.storage.load()?;
        self.rebuild_visible_task_list();
        Ok(())
    }
    
    pub fn input_char(&mut self, c: char) {
        let field = match self.input_state.current_field {
            0 => &mut self.input_state.title,
            1 => &mut self.input_state.description,
            2 => &mut self.input_state.tags,
            3 => &mut self.input_state.estimate,
            4 => &mut self.input_state.assignee,
            5 => &mut self.input_state.note,
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
            4 => &mut self.input_state.assignee,
            5 => &mut self.input_state.note,
            _ => return,
        };
        field.pop();
    }

    pub fn next_field(&mut self) {
        self.input_state.current_field = (self.input_state.current_field + 1).min(5);
    }

    pub fn prev_field(&mut self) {
        self.input_state.current_field = self.input_state.current_field.saturating_sub(1);
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

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
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
                            app.start_add_task();
                        }
                        KeyCode::Char('e') => {
                            app.start_edit_task();
                        }
                        _ => {}
                    }
                }
                AppMode::Help => {
                    if matches!(key.code, KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?')) {
                        app.mode = AppMode::Normal;
                    }
                }
                AppMode::Filter => {
                    if key.code == KeyCode::Esc {
                        app.mode = AppMode::Normal;
                    }
                }
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
                            // If in note field (field 5), insert newline
                            if app.input_state.current_field == 5 {
                                app.input_char('\n');
                            } else if key.modifiers.contains(KeyModifiers::CONTROL) {
                                // Ctrl+Enter saves from any field
                                if matches!(app.mode, AppMode::AddTask) {
                                    let _ = app.save_new_task();
                                } else {
                                    let _ = app.save_edit_task();
                                }
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

