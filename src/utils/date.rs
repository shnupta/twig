use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Utc};

pub fn parse_date(input: &str) -> Result<DateTime<Utc>> {
    let input = input.trim().to_lowercase();

    match input.as_str() {
        "today" => {
            let local = Local::now();
            Ok(local.with_timezone(&Utc))
        }
        "yesterday" => {
            let local = Local::now() - Duration::days(1);
            Ok(local.with_timezone(&Utc))
        }
        "tomorrow" => {
            let local = Local::now() + Duration::days(1);
            Ok(local.with_timezone(&Utc))
        }
        _ => {
            // Try parsing as ISO date (YYYY-MM-DD)
            let naive = NaiveDate::parse_from_str(&input, "%Y-%m-%d").context(
                "Invalid date format. Use YYYY-MM-DD or 'today', 'yesterday', 'tomorrow'",
            )?;
            let dt = Local
                .from_local_datetime(&naive.and_hms_opt(0, 0, 0).unwrap())
                .unwrap();
            Ok(dt.with_timezone(&Utc))
        }
    }
}

pub enum DateRange {
    Day(DateTime<Utc>),
    Week(DateTime<Utc>),  // Start of week
    Month(DateTime<Utc>), // Start of month
}

impl DateRange {
    pub fn parse_day(input: &str) -> Result<Self> {
        let input = input.trim().to_lowercase();
        let date = match input.as_str() {
            "today" => Local::now().with_timezone(&Utc),
            "yesterday" => (Local::now() - Duration::days(1)).with_timezone(&Utc),
            _ => parse_date(&input)?,
        };
        Ok(DateRange::Day(date))
    }

    pub fn parse_week(input: &str) -> Result<Self> {
        let input = input.trim().to_lowercase();
        let date = match input.as_str() {
            "this week" | "week" => {
                let now = Local::now();
                let days_from_monday = now.weekday().num_days_from_monday();
                (now - Duration::days(days_from_monday as i64)).with_timezone(&Utc)
            }
            "last week" => {
                let now = Local::now();
                let days_from_monday = now.weekday().num_days_from_monday();
                (now - Duration::days(days_from_monday as i64 + 7)).with_timezone(&Utc)
            }
            _ => {
                // Parse specific date and get start of that week
                let date = parse_date(&input)?;
                let days_from_monday = date.weekday().num_days_from_monday();
                date - Duration::days(days_from_monday as i64)
            }
        };
        Ok(DateRange::Week(date))
    }

    pub fn parse_month(input: &str) -> Result<Self> {
        let input = input.trim().to_lowercase();
        let date = match input.as_str() {
            "this month" | "month" => {
                let now = Local::now();
                let first_of_month = now.with_day(1).unwrap();
                first_of_month.with_timezone(&Utc)
            }
            "last month" => {
                let now = Local::now();
                let first_of_month = now.with_day(1).unwrap();
                let last_month = first_of_month - Duration::days(1);
                last_month.with_day(1).unwrap().with_timezone(&Utc)
            }
            _ => {
                // Parse specific date and get start of that month
                let date = parse_date(&input)?;
                date.with_day(1).unwrap()
            }
        };
        Ok(DateRange::Month(date))
    }

    pub fn start(&self) -> DateTime<Utc> {
        match self {
            DateRange::Day(d) => d.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc(),
            DateRange::Week(d) => d.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc(),
            DateRange::Month(d) => d.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc(),
        }
    }

    pub fn end(&self) -> DateTime<Utc> {
        match self {
            DateRange::Day(d) => (d.date_naive() + Duration::days(1))
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc(),
            DateRange::Week(d) => (d.date_naive() + Duration::days(7))
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc(),
            DateRange::Month(d) => {
                let next_month = if d.month() == 12 {
                    d.with_year(d.year() + 1).unwrap().with_month(1).unwrap()
                } else {
                    d.with_month(d.month() + 1).unwrap()
                };
                next_month
                    .date_naive()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
            }
        }
    }
}

pub fn format_datetime(dt: &DateTime<Utc>) -> String {
    dt.with_timezone(&Local)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

pub fn format_date(dt: &DateTime<Utc>) -> String {
    dt.with_timezone(&Local).format("%Y-%m-%d").to_string()
}

pub fn format_duration_human(seconds: i64) -> String {
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
