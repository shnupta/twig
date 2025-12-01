use crate::models::TaskStatus;
use crate::tui::app::{App, AppMode, ViewTab, VisibleItemInfo};
use crate::utils::format_datetime;
use chrono::Utc;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    match app.mode {
        AppMode::Help => {
            draw_help(f);
        }
        AppMode::AddTask => {
            draw_main_view(f, app);
            draw_add_task_dialog(f, app);
        }
        AppMode::EditTask => {
            draw_main_view(f, app);
            draw_edit_task_dialog(f, app);
        }
        AppMode::DeleteConfirm => {
            draw_main_view(f, app);
            draw_delete_confirm_dialog(f, app);
        }
        _ => {
            draw_main_view(f, app);
        }
    }
}

fn draw_main_view(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Header (taller for tabs)
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Footer
        ])
        .split(f.area());

    // Header
    draw_header(f, chunks[0], app);

    // Main content - split between task list and details
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    draw_task_list(f, main_chunks[0], app);
    draw_task_details(f, main_chunks[1], app);

    // Footer
    draw_footer(f, chunks[2], app);
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let filters = vec![
        if app.show_completed {
            "✓ Completed"
        } else {
            "✗ Completed"
        },
        if app.show_cancelled {
            "✓ Cancelled"
        } else {
            "✗ Cancelled"
        },
    ];

    let filter_text = filters.join(" | ");

    // Build tab bar
    let mut tab_spans = vec![
        Span::raw("[1] "),
        if matches!(app.view_tab, ViewTab::MyTasks) {
            Span::styled(
                "My Tasks",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )
        } else {
            Span::styled("My Tasks", Style::default())
        },
    ];

    if !app.reportees.is_empty() {
        tab_spans.push(Span::raw("  [2] "));
        if matches!(app.view_tab, ViewTab::AllReportees) {
            tab_spans.push(Span::styled(
                "Reportees",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ));
        } else {
            tab_spans.push(Span::styled("Reportees", Style::default()));
        }
    }

    tab_spans.push(Span::styled(
        "  (1/2 to switch)",
        Style::default().fg(Color::DarkGray),
    ));

    let header = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            "twig",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(tab_spans),
        Line::from(filter_text),
    ])
    .block(Block::default().borders(Borders::ALL));

    f.render_widget(header, area);
}

fn draw_task_list(f: &mut Frame, area: Rect, app: &App) {
    let visible_items = app.get_visible_items();

    let items: Vec<ListItem> = visible_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            match item {
                VisibleItemInfo::ReporteeHeader { name, is_expanded } => {
                    let expand_indicator = if *is_expanded { "▼" } else { "▶" };
                    let content = format!("{} 👤 {}", expand_indicator, name);

                    let style = if i == app.selected_index {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    };

                    ListItem::new(Line::from(Span::styled(content, style)))
                }
                VisibleItemInfo::Task { task, depth, owner } => {
                    let status_icon = match task.status {
                        TaskStatus::NotStarted => "○",
                        TaskStatus::InProgress => "◐",
                        TaskStatus::Completed => "●",
                        TaskStatus::Cancelled => "✗",
                    };

                    let status_color = match task.status {
                        TaskStatus::NotStarted => Color::Gray,
                        TaskStatus::InProgress => Color::Yellow,
                        TaskStatus::Completed => Color::Green,
                        TaskStatus::Cancelled => Color::Red,
                    };

                    // Expand/collapse indicator
                    let expand_indicator = if app.has_children(task.id, owner) {
                        if app.is_expanded(task.id) {
                            "▼ "
                        } else {
                            "▶ "
                        }
                    } else {
                        "  "
                    };

                    // Time tracking status - make it very visible
                    let (time_info, time_color) = if task.has_active_time_entry() {
                        (format!(" ⏱TRACKING"), Some(Color::Yellow))
                    } else if task.status == TaskStatus::InProgress && task.total_time_seconds > 0 {
                        // In progress but not actively tracking = paused
                        (format!(" ⏸PAUSED"), Some(Color::DarkGray))
                    } else if task.total_time_seconds > 0 {
                        (format!(" [{}]", task.get_formatted_total_time()), None)
                    } else {
                        (String::new(), None)
                    };

                    // Indentation for tree structure
                    let indent = "  ".repeat(*depth);

                    let base_content = format!(
                        "{}{}{} {} [{}]",
                        indent,
                        expand_indicator,
                        status_icon,
                        task.title,
                        task.short_id()
                    );

                    // Build the line with styled time tracking info
                    let mut line_spans = vec![Span::raw(base_content)];

                    if !time_info.is_empty() {
                        if let Some(color) = time_color {
                            line_spans.push(Span::styled(
                                time_info,
                                Style::default().fg(color).add_modifier(Modifier::BOLD),
                            ));
                        } else {
                            line_spans.push(Span::raw(time_info));
                        }
                    }

                    let style = if i == app.selected_index {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(status_color)
                    };

                    ListItem::new(Line::from(line_spans)).style(style)
                }
            }
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(format!(
                "Task Tree ({}/{})",
                app.selected_index + 1,
                visible_items.len()
            ))
            .borders(Borders::ALL),
    );

    f.render_widget(list, area);
}

fn draw_task_details(f: &mut Frame, area: Rect, app: &App) {
    if let Some((task, owner)) = app.get_selected_task() {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Title: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&task.title),
            ]),
            Line::from(vec![
                Span::styled("ID: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(task.short_id()),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    match task.status {
                        TaskStatus::NotStarted => "○ Not Started",
                        TaskStatus::InProgress => "◐ In Progress",
                        TaskStatus::Completed => "● Completed",
                        TaskStatus::Cancelled => "✗ Cancelled",
                    },
                    Style::default().fg(match task.status {
                        TaskStatus::NotStarted => Color::Gray,
                        TaskStatus::InProgress => Color::Yellow,
                        TaskStatus::Completed => Color::Green,
                        TaskStatus::Cancelled => Color::Red,
                    }),
                ),
            ]),
        ];

        if !task.description.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Description:",
                Style::default().add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(task.description.clone()));
        }

        if !task.tags.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Tags: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(
                    task.tags
                        .iter()
                        .map(|t| format!("#{}", t))
                        .collect::<Vec<_>>()
                        .join(" "),
                ),
            ]));
        }

        if !task.notes.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Notes:",
                Style::default().add_modifier(Modifier::BOLD),
            )]));
            for line in task.notes.lines() {
                lines.push(Line::from(format!("  {}", line)));
            }
        }

        if let Some(estimate) = task.get_formatted_estimate() {
            lines.push(Line::from(vec![
                Span::styled("Estimate: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(estimate),
            ]));
        }

        if task.total_time_seconds > 0 {
            lines.push(Line::from(vec![
                Span::styled(
                    "Time Spent: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(task.get_formatted_total_time()),
            ]));
        }

        if task.has_active_time_entry() {
            // Calculate how long the current session has been running
            if let Some(last_entry) = task.time_entries.last() {
                if last_entry.end.is_none() {
                    let duration = (Utc::now() - last_entry.start).num_seconds();
                    let hours = duration / 3600;
                    let minutes = (duration % 3600) / 60;
                    lines.push(Line::from(vec![
                        Span::styled(
                            "⏱ ACTIVELY TRACKING ",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("({}h {}m)", hours, minutes),
                            Style::default().fg(Color::Yellow),
                        ),
                    ]));
                }
            }
        } else if task.status == TaskStatus::InProgress {
            lines.push(Line::from(vec![
                Span::styled(
                    "⏸ PAUSED ",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "(press 's' to resume)",
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Created: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format_datetime(&task.created_at)),
        ]));

        if let Some(started) = task.started_at {
            lines.push(Line::from(vec![
                Span::styled("Started: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format_datetime(&started)),
            ]));
        }

        if let Some(completed) = task.completed_at {
            lines.push(Line::from(vec![
                Span::styled("Completed: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format_datetime(&completed)),
            ]));
        }

        if let Some(eta) = task.eta {
            lines.push(Line::from(vec![
                Span::styled("ETA: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format_datetime(&eta)),
            ]));
        }

        let storage = app.get_storage_for_owner(owner);
        let children = storage.get_children(task.id);
        if !children.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                format!("Subtasks ({})", children.len()),
                Style::default().add_modifier(Modifier::BOLD),
            )]));
            for child in children.iter().take(5) {
                let status_icon = match child.status {
                    TaskStatus::NotStarted => "○",
                    TaskStatus::InProgress => "◐",
                    TaskStatus::Completed => "●",
                    TaskStatus::Cancelled => "✗",
                };
                lines.push(Line::from(format!("  {} {}", status_icon, child.title)));
            }
            if children.len() > 5 {
                lines.push(Line::from(format!("  ... and {} more", children.len() - 5)));
            }
        }

        let paragraph = Paragraph::new(lines)
            .block(Block::default().title("Details").borders(Borders::ALL))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    } else {
        let paragraph = Paragraph::new("No task selected")
            .block(Block::default().title("Details").borders(Borders::ALL));
        f.render_widget(paragraph, area);
    }
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let help_text = match app.mode {
        AppMode::Normal => {
            "j/k:↓↑ | Tab/Enter:Expand | ←/→:Tabs | 1-5:Switch tab | s:Start | c:Complete | x:Cancel | p:Pause | a:Add subtask | A:Add top-level | e:Edit | d:Delete | ?:Help | q:Quit"
        }
        AppMode::Help => "Press ? or ESC to close help",
        AppMode::AddTask => "↑/↓/Tab:Navigate | Enter:Activate button or new line | Ctrl+Enter:Save | ESC:Cancel",
        AppMode::EditTask => "↑/↓/Tab:Navigate | Enter:Activate button or new line | Ctrl+Enter:Save | ESC:Cancel",
        AppMode::DeleteConfirm => "Enter/y:Confirm Delete | ESC/n:Cancel",
    };

    let footer = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));

    f.render_widget(footer, area);
}

fn draw_help(f: &mut Frame) {
    let help_text = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "Twig Help",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  j / ↓            - Move down"),
        Line::from("  k / ↑            - Move up"),
        Line::from("  Enter/Space/Tab  - Expand/collapse task (shows/hides subtasks)"),
        Line::from("  ← / →            - Switch tabs (My Tasks / Reportees)"),
        Line::from("  1-5              - Jump to specific tab"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Task Management",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  a       - Add new task (as subtask of selected)"),
        Line::from("  A       - Add new task (as top-level, not a subtask)"),
        Line::from("  e       - Edit selected task"),
        Line::from("  d       - Delete selected task (with confirmation)"),
        Line::from("  s - Start task (begins time tracking)"),
        Line::from("  c - Complete task (stops time tracking)"),
        Line::from("  x - Cancel task"),
        Line::from("  p - Pause time tracking (keeps task in progress)"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Filters",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  h - Toggle show/hide completed"),
        Line::from("  H - Toggle show/hide cancelled"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Other",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  r       - Reload tasks from disk"),
        Line::from("  ?       - Toggle help"),
        Line::from("  q       - Quit"),
        Line::from("  Ctrl+C  - Quit"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Tree View Indicators",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  ▶ - Collapsed (has children, not showing)"),
        Line::from("  ▼ - Expanded (has children, showing)"),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "⏱TRACKING",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" - Timer actively running"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "⏸PAUSED",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   - In progress but timer stopped"),
        ]),
        Line::from(""),
        Line::from("Press ? or ESC to close help"),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(Block::default().title("Help").borders(Borders::ALL))
        .wrap(Wrap { trim: true });

    let area = centered_rect(60, 80, f.area());
    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_delete_confirm_dialog(f: &mut Frame, app: &App) {
    if let Some(task_id) = app.editing_task_id {
        if let Some((task, owner)) = app.get_task_by_id_with_owner(task_id) {
            let area = centered_rect(60, 30, f.area());

            // Clear background
            f.render_widget(ratatui::widgets::Clear, area);

            let storage = app.get_storage_for_owner(owner);
            let children = storage.get_children(task_id);
            let has_subtasks = !children.is_empty();

            let warning_text = if has_subtasks {
                format!(
                    "Delete task \"{}\"?\n\n⚠ WARNING: This task has {} subtask(s)!\nDeleting it will NOT delete the subtasks,\nbut they will become orphaned.\n\nAre you sure?",
                    task.title,
                    children.len()
                )
            } else {
                format!(
                    "Delete task \"{}\"?\n\nThis action cannot be undone.",
                    task.title
                )
            };

            let warning_style = if has_subtasks {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Yellow)
            };

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([
                    Constraint::Min(5),    // Warning text
                    Constraint::Length(3), // Buttons
                ])
                .split(area);

            let block = Block::default()
                .title("Confirm Delete")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black));
            f.render_widget(block, area);

            let warning = Paragraph::new(warning_text)
                .style(warning_style)
                .wrap(Wrap { trim: true })
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(warning, chunks[0]);

            // Buttons
            let button_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[1]);

            let delete_button = Paragraph::new("[ Delete ] (Enter/y)")
                .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
                .alignment(ratatui::layout::Alignment::Center)
                .block(Block::default().borders(Borders::ALL));

            let cancel_button = Paragraph::new("[ Cancel ] (ESC/n)")
                .style(Style::default().fg(Color::Green))
                .alignment(ratatui::layout::Alignment::Center)
                .block(Block::default().borders(Borders::ALL));

            f.render_widget(delete_button, button_chunks[0]);
            f.render_widget(cancel_button, button_chunks[1]);
        }
    }
}

fn draw_add_task_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(80, 75, f.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Description
            Constraint::Length(3), // Tags
            Constraint::Length(3), // Estimate
            Constraint::Min(5),    // Note (multiline)
            Constraint::Length(3), // Buttons
            Constraint::Length(2), // Info
        ])
        .split(area);

    // Clear background
    let block = Block::default()
        .title("Add New Task")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));
    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(block, area);

    // Regular single-line fields
    let single_line_fields = [
        ("Title*", &app.input_state.title, 0, 0),
        ("Description", &app.input_state.description, 1, 1),
        ("Tags (comma-separated)", &app.input_state.tags, 2, 2),
        ("Estimate (1h/2d/3w/2m)", &app.input_state.estimate, 3, 3),
    ];

    for (label, value, field_idx, chunk_idx) in single_line_fields.iter() {
        let style = if app.input_state.current_field == *field_idx {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let input = Paragraph::new(format!("{}: {}", label, value))
            .style(style)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(input, chunks[*chunk_idx]);
    }

    // Multiline note field
    let note_style = if app.input_state.current_field == 4 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let note_text = if app.input_state.note.is_empty() {
        "Notes (multiline - press Enter for new line):".to_string()
    } else {
        format!("Notes:\n{}", app.input_state.note)
    };

    let note_input = Paragraph::new(note_text)
        .style(note_style)
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(note_input, chunks[4]);

    // Buttons
    let button_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[5]);

    let save_style = if app.input_state.current_field == 5 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };

    let cancel_style = if app.input_state.current_field == 6 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Red)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red)
    };

    let save_button = Paragraph::new("[ Save ]")
        .style(save_style)
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    let cancel_button = Paragraph::new("[ Cancel ]")
        .style(cancel_style)
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(save_button, button_chunks[0]);
    f.render_widget(cancel_button, button_chunks[1]);

    // Help text
    let parent_info = if app.editing_task_id.is_some() {
        "Will be added as subtask of selected task"
    } else {
        "Will be added as top-level task"
    };
    let help = Paragraph::new(format!(
        "↑/↓/Tab:Navigate | Enter:Select button or new line | Ctrl+Enter:Save | ESC:Cancel\n{}",
        parent_info
    ))
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[6]);
}

fn draw_edit_task_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(80, 75, f.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Description
            Constraint::Length(3), // Tags
            Constraint::Length(3), // Estimate
            Constraint::Min(5),    // Note (multiline)
            Constraint::Length(3), // Buttons
            Constraint::Length(2), // Info
        ])
        .split(area);

    // Clear background
    let block = Block::default()
        .title("Edit Task")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));
    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(block, area);

    // Regular single-line fields
    let single_line_fields = [
        ("Title*", &app.input_state.title, 0, 0),
        ("Description", &app.input_state.description, 1, 1),
        ("Tags (comma-separated)", &app.input_state.tags, 2, 2),
        ("Estimate (1h/2d/3w/2m)", &app.input_state.estimate, 3, 3),
    ];

    for (label, value, field_idx, chunk_idx) in single_line_fields.iter() {
        let style = if app.input_state.current_field == *field_idx {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let input = Paragraph::new(format!("{}: {}", label, value))
            .style(style)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(input, chunks[*chunk_idx]);
    }

    // Multiline note field
    let note_style = if app.input_state.current_field == 4 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let note_text = if app.input_state.note.is_empty() {
        "Notes (multiline - press Enter for new line):".to_string()
    } else {
        format!("Notes:\n{}", app.input_state.note)
    };

    let note_input = Paragraph::new(note_text)
        .style(note_style)
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(note_input, chunks[4]);

    // Buttons
    let button_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[5]);

    let save_style = if app.input_state.current_field == 5 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };

    let cancel_style = if app.input_state.current_field == 6 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Red)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red)
    };

    let save_button = Paragraph::new("[ Save ]")
        .style(save_style)
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    let cancel_button = Paragraph::new("[ Cancel ]")
        .style(cancel_style)
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(save_button, button_chunks[0]);
    f.render_widget(cancel_button, button_chunks[1]);

    // Help text
    let help = Paragraph::new("↑/↓/Tab:Navigate | Enter:Select button or new line (in note) | Ctrl+Enter:Save | ESC:Cancel")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[6]);
}
