use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

#[derive(Parser)]
#[command(name = "twig")]
#[command(about = "A terminal task tracking application", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a new task
    Add {
        /// Task title
        title: String,

        /// Parent task ID (short or full UUID)
        #[arg(short, long)]
        parent: Option<String>,

        /// Tags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,

        /// Estimated effort (e.g., "1h", "2d", "3w", "2m")
        #[arg(short, long)]
        estimate: Option<String>,

        /// Estimated completion date (YYYY-MM-DD)
        #[arg(long)]
        eta: Option<String>,

        /// Task description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Start working on a task (interactive selector)
    Start,

    /// Complete a task (interactive selector)
    Complete,

    /// Cancel a task (interactive selector)
    Cancel,

    /// Pause active time tracking on a task (interactive selector)
    Pause,

    /// List tasks
    List {
        /// Filter by status
        #[arg(short, long)]
        status: Option<StatusFilter>,

        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,
    },

    /// Show detailed information about a task (interactive selector)
    Show,

    /// Display task tree
    Tree,

    /// Update task fields (interactive selector)
    Update {
        /// New title
        #[arg(long)]
        title: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,

        /// New estimated effort (e.g., "1h", "2d", "3w", "2m")
        #[arg(long)]
        estimate: Option<String>,

        /// New ETA (YYYY-MM-DD)
        #[arg(long)]
        eta: Option<String>,
    },

    /// Delete a task (interactive selector)
    Delete,

    /// Add tags to a task (interactive selector)
    Tag {
        /// Tags to add
        tags: Vec<String>,
    },

    /// Manage reportees
    Reportee {
        #[command(subcommand)]
        command: ReporteeCommands,
    },

    /// Generate reports
    Report {
        /// Report period
        #[arg(value_enum)]
        period: ReportPeriod,

        /// Specific date (YYYY-MM-DD, or "today", "yesterday", "this week", etc.)
        #[arg(short, long)]
        date: Option<String>,
    },

    /// Show statistics
    Stats {
        /// Statistics period
        #[arg(value_enum)]
        period: Option<StatsPeriod>,

        /// Specific date (YYYY-MM-DD, or "today", "yesterday", "this week", etc.)
        #[arg(short, long)]
        date: Option<String>,
    },

    /// Launch interactive TUI
    Tui,

    /// Generate shell completions
    Completions {
        /// Shell type
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Subcommand)]
pub enum ReporteeCommands {
    /// Add a reportee
    Add {
        /// Reportee name
        name: String,
    },

    /// List all reportees
    List,

    /// Remove a reportee
    Remove {
        /// Reportee name
        name: String,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum StatusFilter {
    NotStarted,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum ReportPeriod {
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum StatsPeriod {
    Daily,
    Weekly,
    Monthly,
}
