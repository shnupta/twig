use crate::cli::{ReportPeriod, StatsPeriod};
use crate::models::{Task, TaskStatus};
use crate::storage::{DataPaths, Storage};
use crate::utils::date::{DateRange, format_date, format_datetime, format_duration_human};
use anyhow::Result;
use comfy_table::{presets::UTF8_FULL, Cell, ContentArrangement, Table};

pub fn generate_report(
    period: ReportPeriod,
    date: Option<String>,
    assignee: Option<String>,
) -> Result<()> {
    let paths = DataPaths::new()?;
    let mut storage = Storage::new(paths.tasks_file().to_string_lossy().to_string());
    storage.load()?;

    let date_str = date.unwrap_or_else(|| "today".to_string());
    let range = match period {
        ReportPeriod::Daily => DateRange::parse_day(&date_str)?,
        ReportPeriod::Weekly => DateRange::parse_week(&date_str)?,
        ReportPeriod::Monthly => DateRange::parse_month(&date_str)?,
    };

    let start = range.start();
    let end = range.end();

    println!("\n{} Report", match period {
        ReportPeriod::Daily => "Daily",
        ReportPeriod::Weekly => "Weekly",
        ReportPeriod::Monthly => "Monthly",
    });
    println!("Period: {} to {}", format_date(&start), format_date(&end));
    if let Some(ref a) = assignee {
        println!("Assignee: @{}", a);
    }
    println!("{}", "=".repeat(60));

    let tasks = storage.get_all_tasks();
    
    // Filter by assignee
    let tasks: Vec<&Task> = if let Some(ref a) = assignee {
        tasks.iter().filter(|t| t.assigned_to.as_deref() == Some(a)).collect()
    } else {
        tasks.iter().collect()
    };

    // Tasks created in period
    let created: Vec<&Task> = tasks
        .iter()
        .filter(|t| t.created_at >= start && t.created_at < end)
        .copied()
        .collect();

    // Tasks started in period
    let started: Vec<&Task> = tasks
        .iter()
        .filter(|t| {
            if let Some(started_at) = t.started_at {
                started_at >= start && started_at < end
            } else {
                false
            }
        })
        .copied()
        .collect();

    // Tasks completed in period
    let completed: Vec<&Task> = tasks
        .iter()
        .filter(|t| {
            if let Some(completed_at) = t.completed_at {
                completed_at >= start && completed_at < end
            } else {
                false
            }
        })
        .copied()
        .collect();

    // Tasks cancelled in period
    let cancelled: Vec<&Task> = tasks
        .iter()
        .filter(|t| {
            if let Some(cancelled_at) = t.cancelled_at {
                cancelled_at >= start && cancelled_at < end
            } else {
                false
            }
        })
        .copied()
        .collect();

    // Tasks in progress during period
    let in_progress: Vec<&Task> = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::InProgress)
        .copied()
        .collect();

    println!("\nSummary:");
    println!("  Created:     {} task(s)", created.len());
    println!("  Started:     {} task(s)", started.len());
    println!("  Completed:   {} task(s)", completed.len());
    println!("  Cancelled:   {} task(s)", cancelled.len());
    println!("  In Progress: {} task(s)", in_progress.len());

    if !completed.is_empty() {
        println!("\nCompleted Tasks:");
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec!["Title", "ID", "Time Spent", "Completed At"]);

        for task in &completed {
            table.add_row(vec![
                Cell::new(&task.title),
                Cell::new(task.short_id()),
                Cell::new(if task.total_time_seconds > 0 {
                    task.get_formatted_total_time()
                } else {
                    String::from("-")
                }),
                Cell::new(format_datetime(&task.completed_at.unwrap())),
            ]);
        }
        println!("{}", table);
    }

    if !in_progress.is_empty() {
        println!("\nIn Progress:");
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec!["Title", "ID", "Time Spent", "Started At"]);

        for task in &in_progress {
            table.add_row(vec![
                Cell::new(&task.title),
                Cell::new(task.short_id()),
                Cell::new(if task.total_time_seconds > 0 {
                    task.get_formatted_total_time()
                } else {
                    String::from("-")
                }),
                Cell::new(if let Some(started) = task.started_at {
                    format_datetime(&started)
                } else {
                    String::from("-")
                }),
            ]);
        }
        println!("{}", table);
    }

    println!("{}", "=".repeat(60));

    Ok(())
}

pub fn show_stats(
    period: Option<StatsPeriod>,
    date: Option<String>,
    assignee: Option<String>,
) -> Result<()> {
    let paths = DataPaths::new()?;
    let mut storage = Storage::new(paths.tasks_file().to_string_lossy().to_string());
    storage.load()?;

    let tasks = storage.get_all_tasks();

    // Filter by assignee
    let tasks: Vec<&Task> = if let Some(ref a) = assignee {
        tasks.iter().filter(|t| t.assigned_to.as_deref() == Some(a)).collect()
    } else {
        tasks.iter().collect()
    };

    // If period and date specified, filter by date range
    let (tasks, period_info): (Vec<&Task>, Option<String>) = if let Some(p) = period {
        let date_str = date.unwrap_or_else(|| "today".to_string());
        let range = match p {
            StatsPeriod::Daily => DateRange::parse_day(&date_str)?,
            StatsPeriod::Weekly => DateRange::parse_week(&date_str)?,
            StatsPeriod::Monthly => DateRange::parse_month(&date_str)?,
        };

        let start = range.start();
        let end = range.end();
        
        let filtered: Vec<&Task> = tasks
            .into_iter()
            .filter(|t| {
                // Include task if it was created, started, or completed in the range
                (t.created_at >= start && t.created_at < end)
                    || (t.started_at.map_or(false, |s| s >= start && s < end))
                    || (t.completed_at.map_or(false, |c| c >= start && c < end))
            })
            .collect();

        (
            filtered,
            Some(format!("{} to {}", format_date(&start), format_date(&end))),
        )
    } else {
        (tasks, None)
    };

    println!("\nStatistics");
    if let Some(ref a) = assignee {
        println!("Assignee: @{}", a);
    }
    if let Some(ref info) = period_info {
        println!("Period: {}", info);
    }
    println!("{}", "=".repeat(60));

    let total = tasks.len();
    let not_started = tasks.iter().filter(|t| t.status == TaskStatus::NotStarted).count();
    let in_progress = tasks.iter().filter(|t| t.status == TaskStatus::InProgress).count();
    let completed = tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();
    let cancelled = tasks.iter().filter(|t| t.status == TaskStatus::Cancelled).count();

    println!("\nTask Status:");
    println!("  Total:        {}", total);
    println!("  Not Started:  {} ({:.1}%)", not_started, (not_started as f64 / total as f64) * 100.0);
    println!("  In Progress:  {} ({:.1}%)", in_progress, (in_progress as f64 / total as f64) * 100.0);
    println!("  Completed:    {} ({:.1}%)", completed, (completed as f64 / total as f64) * 100.0);
    println!("  Cancelled:    {} ({:.1}%)", cancelled, (cancelled as f64 / total as f64) * 100.0);

    // Time statistics
    let total_time: i64 = tasks.iter().map(|t| t.total_time_seconds).sum();
    let avg_time = if !tasks.is_empty() {
        total_time / tasks.len() as i64
    } else {
        0
    };

    println!("\nTime Tracking:");
    println!("  Total Time:   {}", format_duration_human(total_time));
    println!("  Average Time: {}", format_duration_human(avg_time));

    // Estimate vs actual for completed tasks
    let completed_tasks: Vec<&&Task> = tasks.iter().filter(|t| t.status == TaskStatus::Completed).collect();
    if !completed_tasks.is_empty() {
        let with_estimates: Vec<&&Task> = completed_tasks
            .iter()
            .filter(|t| t.estimated_effort_hours.is_some())
            .copied()
            .collect();

        if !with_estimates.is_empty() {
            let total_estimated: f64 = with_estimates
                .iter()
                .map(|t| t.estimated_effort_hours.unwrap_or(0.0))
                .sum();
            let total_actual: f64 = with_estimates
                .iter()
                .map(|t| t.total_time_seconds as f64 / 3600.0)
                .sum();

            println!("\nEstimate Accuracy (Completed Tasks with Estimates):");
            println!("  Estimated: {:.1}h", total_estimated);
            println!("  Actual:    {:.1}h", total_actual);
            println!("  Variance:  {:.1}%", ((total_actual - total_estimated) / total_estimated) * 100.0);
        }
    }

    // Tags analysis
    let mut tag_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for task in &tasks {
        for tag in &task.tags {
            *tag_counts.entry(tag.clone()).or_insert(0) += 1;
        }
    }

    if !tag_counts.is_empty() {
        println!("\nTop Tags:");
        let mut tags: Vec<_> = tag_counts.iter().collect();
        tags.sort_by(|a, b| b.1.cmp(a.1));
        for (tag, count) in tags.iter().take(10) {
            println!("  #{}: {}", tag, count);
        }
    }

    println!("{}", "=".repeat(60));

    Ok(())
}

