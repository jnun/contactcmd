use anyhow::{bail, Result};
use chrono::{DateTime, Duration, Local, NaiveDate, TimeZone, Utc};

use crate::db::Database;
use crate::models::Person;
use super::CheckinCommands;
use super::ui::{find_person_by_identifier, get_display_name};

/// Parse a date string into a UTC datetime.
/// Supports: "today", "tomorrow", "+Nd" (days), "+Nw" (weeks), YYYY-MM-DD
fn parse_date(s: &str) -> Result<DateTime<Utc>> {
    let s = s.trim().to_lowercase();
    let today = Local::now().date_naive();

    let date = match s.as_str() {
        "today" => today,
        "tomorrow" => today + Duration::days(1),
        _ if s.starts_with('+') => {
            let (n, days_per_unit) = parse_relative(&s[1..])?;
            today + Duration::days(n * days_per_unit)
        }
        _ => NaiveDate::parse_from_str(&s, "%Y-%m-%d")
            .map_err(|_| anyhow::anyhow!("Invalid date. Use: today, tomorrow, +3d, +1w, or YYYY-MM-DD"))?,
    };

    // Convert to 9am local time, then UTC
    Local.from_local_datetime(&date.and_hms_opt(9, 0, 0).unwrap())
        .single()
        .map(|dt| dt.with_timezone(&Utc))
        .ok_or_else(|| anyhow::anyhow!("Invalid datetime"))
}

/// Parse relative date suffix like "3d" or "2w". Returns (number, days_per_unit).
fn parse_relative(s: &str) -> Result<(i64, i64)> {
    let (num_str, multiplier) = match s.chars().last() {
        Some('d') => (&s[..s.len()-1], 1),
        Some('w') => (&s[..s.len()-1], 7),
        _ => bail!("Use +Nd or +Nw (e.g., +3d, +1w)"),
    };
    let n: i64 = num_str.parse().map_err(|_| anyhow::anyhow!("Invalid number: {}", num_str))?;
    if n < 0 { bail!("Number must be positive"); }
    Ok((n, multiplier))
}

/// Format a checkin date relative to today.
fn format_date(date: DateTime<Utc>) -> String {
    let days = (date.with_timezone(&Local).date_naive() - Local::now().date_naive()).num_days();
    match days {
        d if d < -1 => format!("{} days overdue", -d),
        -1 => "1 day overdue".into(),
        0 => "today".into(),
        1 => "tomorrow".into(),
        2..=7 => format!("in {} days", days),
        _ => date.with_timezone(&Local).format("%Y-%m-%d").to_string(),
    }
}

/// Print a single checkin row: "  [marker] Name - date"
fn print_checkin(person: &Person, mark_overdue: bool) {
    let name = get_display_name(person);
    // checkin_date is always Some here (query filters for NOT NULL)
    let date = person.checkin_date.unwrap();
    let is_due = date <= Utc::now();
    let marker = if mark_overdue && is_due { "[!]" } else { "[ ]" };
    println!("  {} {} - {}", marker, name, format_date(date));
}

pub fn run_checkin(db: &Database, command: CheckinCommands) -> Result<()> {
    match command {
        CheckinCommands::List => {
            let checkins = db.get_checkins_due()?;
            if checkins.is_empty() {
                println!("No checkins due.");
            } else {
                println!("Due ({}):\n", checkins.len());
                for person in &checkins {
                    print_checkin(person, false);
                }
            }
        }

        CheckinCommands::All => {
            let checkins = db.get_all_checkins()?;
            if checkins.is_empty() {
                println!("No checkins scheduled.");
            } else {
                println!("All checkins ({}):\n", checkins.len());
                for person in &checkins {
                    print_checkin(person, true);
                }
            }
        }

        CheckinCommands::Set { identifier, date } => {
            let person = find_person_by_identifier(db, &identifier)?
                .ok_or_else(|| anyhow::anyhow!("Not found: {}", identifier))?;
            let date = parse_date(&date)?;
            db.set_checkin_date(person.id, date)?;
            println!("{} - {}", get_display_name(&person), format_date(date));
        }

        CheckinCommands::Done { identifier } => {
            let person = find_person_by_identifier(db, &identifier)?
                .ok_or_else(|| anyhow::anyhow!("Not found: {}", identifier))?;
            let name = get_display_name(&person);

            if person.checkin_date.is_none() {
                println!("{} has no checkin.", name);
                return Ok(());
            }

            db.clear_checkin_date(person.id)?;
            println!("Done: {}", name);
        }
    }
    Ok(())
}
