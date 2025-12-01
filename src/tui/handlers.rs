// Event handlers for TUI
// Currently handled inline in app.rs, but can be moved here for organization

use crate::tui::app::App;
use anyhow::Result;

pub fn handle_start_task(app: &mut App) -> Result<()> {
    app.start_selected_task()
}

pub fn handle_complete_task(app: &mut App) -> Result<()> {
    app.complete_selected_task()
}

pub fn handle_cancel_task(app: &mut App) -> Result<()> {
    app.cancel_selected_task()
}

pub fn handle_pause_task(app: &mut App) -> Result<()> {
    app.pause_selected_task()
}
