use crate::models::ContactDetail;
use super::messages::LastMessage;
use chrono::{Datelike, Local, Timelike};

/// Print a full contact detail with clean formatting (only non-empty fields)
pub fn print_full_contact(detail: &ContactDetail, last_message: Option<&LastMessage>) {
    let person = &detail.person;
    let display_name = person
        .display_name
        .as_deref()
        .unwrap_or("(unnamed)");

    // Header - just the name
    println!("{}\n", display_name);

    // Organization
    for (po, org) in &detail.organizations {
        let title_part = po.title.as_ref().map(|t| format!("{} at ", t)).unwrap_or_default();
        println!("  {}{}", title_part, org.name);
    }

    // Emails
    for email in &detail.emails {
        println!("  {}", email.email_address);
    }

    // Phones
    for phone in &detail.phones {
        println!("  {}", phone.phone_number);
    }

    // Addresses - city, state only for brevity
    for addr in &detail.addresses {
        if let Some(loc) = addr.city_state() {
            println!("  {}", loc);
        }
    }

    // Notes (truncated)
    if let Some(ref notes) = person.notes {
        if !notes.is_empty() {
            let truncated = if notes.chars().count() > 60 {
                let text: String = notes.chars().take(59).collect();
                format!("{}…", text.trim_end())
            } else {
                notes.clone()
            };
            println!("  {}", truncated);
        }
    }

    // Last Message
    if let Some(msg) = last_message {
        let date_str = format_message_date(&msg.date);
        let direction = if msg.is_from_me { ">" } else { "<" };
        let text = truncate_message(&msg.text, 40);
        println!("\n  {} {} \"{}\"", direction, date_str, text);
    }
}

/// Format a message date for display
pub fn format_message_date(date: &chrono::DateTime<Local>) -> String {
    let now = Local::now();
    let today = now.date_naive();
    let msg_date = date.date_naive();

    if msg_date == today {
        // Today: show time only
        format!("Today at {}:{:02}{}",
            date.hour12().1,
            date.minute(),
            if date.hour12().0 { "pm" } else { "am" }
        )
    } else if msg_date == today.pred_opt().unwrap_or(today) {
        // Yesterday
        format!("Yesterday at {}:{:02}{}",
            date.hour12().1,
            date.minute(),
            if date.hour12().0 { "pm" } else { "am" }
        )
    } else if date.year() == now.year() {
        // This year: show month and day
        format!("{} {} at {}:{:02}{}",
            month_abbrev(date.month()),
            date.day(),
            date.hour12().1,
            date.minute(),
            if date.hour12().0 { "pm" } else { "am" }
        )
    } else {
        // Different year: show full date
        format!("{} {}, {} at {}:{:02}{}",
            month_abbrev(date.month()),
            date.day(),
            date.year(),
            date.hour12().1,
            date.minute(),
            if date.hour12().0 { "pm" } else { "am" }
        )
    }
}

/// Get month abbreviation
fn month_abbrev(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???",
    }
}

/// Truncate message text to first line and max length
fn truncate_message(text: &str, max_len: usize) -> String {
    // Take first line only
    let first_line = text.lines().next().unwrap_or("");

    // Trim whitespace
    let trimmed = first_line.trim();

    if trimmed.chars().count() <= max_len {
        trimmed.to_string()
    } else {
        let truncated: String = trimmed.chars().take(max_len - 1).collect();
        format!("{}…", truncated.trim_end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        Address, AddressType, Email, EmailType, Person, Phone, PhoneType,
        SpecialDate, DateType,
    };

    fn make_test_detail() -> ContactDetail {
        let mut person = Person::new();
        person.name_given = Some("John".to_string());
        person.name_family = Some("Smith".to_string());
        person.name_nickname = Some("Johnny".to_string());
        person.notes = Some("Met at tech conference".to_string());
        person.compute_names();

        let mut email = Email::new(person.id, "john@example.com".to_string());
        email.email_type = EmailType::Work;
        email.is_primary = true;

        let mut phone = Phone::new(person.id, "(555) 123-4567".to_string());
        phone.phone_type = PhoneType::Mobile;
        phone.is_primary = true;

        let mut addr = Address::new(person.id);
        addr.street = Some("123 Main St".to_string());
        addr.city = Some("Austin".to_string());
        addr.state = Some("TX".to_string());
        addr.postal_code = Some("78701".to_string());
        addr.address_type = AddressType::Home;
        addr.is_primary = true;

        let birthday = SpecialDate::new(
            person.id,
            "1985-03-15".to_string(),
            DateType::Birthday,
        );

        ContactDetail {
            person,
            emails: vec![email],
            phones: vec![phone],
            addresses: vec![addr],
            organizations: vec![],
            tags: vec![],
            special_dates: vec![birthday],
            notes: vec![],
            interactions: vec![],
        }
    }

    #[test]
    fn test_print_full_contact_does_not_panic() {
        let detail = make_test_detail();
        print_full_contact(&detail, None);
    }
}
