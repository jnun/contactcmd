use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    style::{Attribute, SetAttribute},
    ExecutableCommand,
};
use rfd::FileDialog;
use std::io::{self, IsTerminal, Write};

use crate::cli::display::print_full_contact_with_tasks;
use crate::cli::photo_utils;
use crate::cli::show::show_messages_screen;
use crate::cli::task::run_tasks_for_contact;
use crate::cli::ui::{
    clear_screen, confirm, is_valid_email, prompt_field, prompt_undo, select, show_help,
    task_action_label, visible_lines, FormResult, RawModeGuard, StatusBar, truncate,
};
use crate::models::{NameOrder, PersonType};
#[cfg(target_os = "macos")]
use crate::cli::sync::{delete_from_macos_contacts, get_apple_id};
use crate::db::Database;
use crate::models::PersonOrganization;

/// View mode for unified browse function
#[derive(Clone, Copy, PartialEq, Default)]
pub enum ViewMode {
    #[default]
    Card,
    Table,
}

/// Contact data prepared for list display
pub struct ContactListRow {
    pub id: uuid::Uuid,
    pub display_name: String,
    pub title_and_org: Option<String>,
    pub primary_email: Option<String>,
    pub primary_phone: Option<String>,
    pub location: Option<String>,
}

/// Sort field for list command
#[derive(Debug, Clone, Copy)]
pub enum SortField {
    Name,
    Created,
    Updated,
}

impl SortField {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "name" => Some(Self::Name),
            "created" => Some(Self::Created),
            "updated" => Some(Self::Updated),
            _ => None,
        }
    }

    pub fn to_sql_column(&self) -> &'static str {
        match self {
            Self::Name => "sort_name",
            Self::Created => "created_at",
            Self::Updated => "updated_at",
        }
    }
}

/// Sort direction
#[derive(Debug, Clone, Copy)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl SortOrder {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "asc" => Some(Self::Asc),
            "desc" => Some(Self::Desc),
            _ => None,
        }
    }

    pub fn to_sql(&self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}


/// Execute the list command
pub fn run_list(
    db: &Database,
    _page: u32,
    _limit: u32,
    sort: Option<String>,
    order: String,
    all: bool,
    review: bool,
) -> Result<()> {
    let sort_field = sort
        .as_deref()
        .and_then(SortField::parse)
        .unwrap_or(SortField::Name);

    let sort_order = SortOrder::parse(&order).unwrap_or(SortOrder::Asc);

    let total = db.count_persons()?;

    if total == 0 {
        println!("No contacts.");
        return Ok(());
    }

    // Review mode - interactive browse
    if review {
        let persons = db.list_persons_sorted(
            total,
            0,
            sort_field.to_sql_column(),
            sort_order.to_sql(),
        )?;
        return run_browse(db, persons, ViewMode::Card);
    }

    // Use non-interactive mode if --all flag or not a TTY
    if all || !io::stdout().is_terminal() {
        let rows =
            db.list_contact_rows(total, 0, sort_field.to_sql_column(), sort_order.to_sql())?;
        println!("Contacts ({} total)\n", total);
        print_table_header();
        for row in rows {
            print_contact_row(&row, false);
        }
        return Ok(());
    }

    // Interactive table mode
    let persons = db.list_persons_sorted(
        total,
        0,
        sort_field.to_sql_column(),
        sort_order.to_sql(),
    )?;
    run_browse(db, persons, ViewMode::Table)
}

/// Get terminal width, defaulting to 80 if unavailable
fn get_term_width() -> usize {
    crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80)
}

/// Column layout based on terminal width
struct ColumnLayout {
    name_width: usize,
    contact_width: usize,
    show_location: bool,
}

impl ColumnLayout {
    fn for_width(width: usize) -> Self {
        if width >= 80 {
            // Full display: Name | Email/Phone | Location
            ColumnLayout {
                name_width: 30,
                contact_width: 28,
                show_location: true,
            }
        } else {
            // Compact display: Name | Email/Phone
            ColumnLayout {
                name_width: 25,
                contact_width: width.saturating_sub(27),
                show_location: false,
            }
        }
    }
}

fn print_table_header() {
    let layout = ColumnLayout::for_width(get_term_width());
    if layout.show_location {
        println!(
            "{:<name_w$}  {:<contact_w$}  LOCATION",
            "NAME",
            "EMAIL/PHONE",
            name_w = layout.name_width,
            contact_w = layout.contact_width
        );
    } else {
        println!(
            "{:<name_w$}  EMAIL/PHONE",
            "NAME",
            name_w = layout.name_width
        );
    }
}

fn print_contact_row(row: &ContactListRow, selected: bool) {
    let layout = ColumnLayout::for_width(get_term_width());
    let name = truncate(&row.display_name, layout.name_width);

    let contact_info = match (&row.primary_email, &row.primary_phone) {
        (Some(email), _) => truncate(email, layout.contact_width),
        (None, Some(phone)) => truncate(phone, layout.contact_width),
        (None, None) => String::new(),
    };

    let line = if layout.show_location {
        let location = row
            .location
            .as_ref()
            .map(|l| truncate(l, 20))
            .unwrap_or_default();
        format!(
            "{:<name_w$}  {:<contact_w$}  {}",
            name,
            contact_info,
            location,
            name_w = layout.name_width,
            contact_w = layout.contact_width
        )
    } else {
        format!(
            "{:<name_w$}  {}",
            name,
            contact_info,
            name_w = layout.name_width
        )
    };

    if selected {
        let mut stdout = io::stdout();
        let _ = stdout.execute(SetAttribute(Attribute::Reverse));
        print!("{}", line);
        let _ = stdout.execute(SetAttribute(Attribute::Reset));
        println!();
    } else {
        println!("{}", line);
    }
}

// ============================================================================
// Unified Browse Mode
// ============================================================================

/// Unified browse mode with card/table view toggle
pub fn run_browse(db: &Database, persons: Vec<crate::models::Person>, initial_mode: ViewMode) -> Result<()> {
    if persons.is_empty() {
        println!("No contacts to browse.");
        return Ok(());
    }

    let mut index: usize = 0;
    let mut scroll: usize = 0;
    let mut view_mode = initial_mode;
    let total = persons.len();

    loop {
        clear_screen()?;

        match view_mode {
            ViewMode::Card => {
                // Card view - show full contact detail
                let person = &persons[index];
                let detail = match db.get_contact_detail(person.id)? {
                    Some(d) => d,
                    None => {
                        index = (index + 1).min(total - 1);
                        continue;
                    }
                };

                let pending_tasks = db.get_pending_tasks_for_person(detail.person.id, 3)?;
                print_full_contact_with_tasks(&detail, None, &pending_tasks);

                let pending_count = pending_tasks.len() as u32;
                let status = StatusBar::new()
                    .counter(index + 1, total)
                    .action("e", "dit")
                    .action("m", "sg")
                    .action("n", "ote")
                    .action("t", &task_action_label(pending_count))
                    .action("d", "el")
                    .action("v", "iew")
                    .separator()
                    .action("?", "")
                    .action("q", "")
                    .action("Q", "uit")
                    .render();
                print!("\n{}: ", status);
                io::stdout().flush()?;

                let action = {
                    let _guard = RawModeGuard::new()?;
                    match event::read()? {
                        Event::Key(KeyEvent { code, .. }) => code,
                        _ => continue,
                    }
                };

                match action {
                    KeyCode::Char('e') | KeyCode::Char('E') => {
                        println!();
                        let detail = db.get_contact_detail(person.id)?.unwrap();
                        handle_full_edit(db, &detail)?;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        println!();
                        let detail = db.get_contact_detail(person.id)?.unwrap();
                        handle_notes(db, &detail)?;
                    }
                    KeyCode::Char('m') | KeyCode::Char('M') => {
                        println!();
                        let detail = db.get_contact_detail(person.id)?.unwrap();
                        if show_messages_screen(db, &detail)? {
                            break;
                        }
                    }
                    KeyCode::Char('t') | KeyCode::Char('T') => {
                        println!();
                        let person_name = person.display_name.as_deref().unwrap_or("(unnamed)");
                        if run_tasks_for_contact(db, person.id, person_name)? {
                            break;
                        }
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') => {
                        let display_name = person.display_name.as_deref().unwrap_or("(unnamed)");
                        println!();
                        if confirm(&format!("Delete \"{}\"?", display_name))? {
                            // Backup before delete for undo
                            let backup = db.get_contact_detail(person.id)?;

                            #[cfg(target_os = "macos")]
                            if let Some(apple_id) = get_apple_id(person) {
                                if let Err(e) = delete_from_macos_contacts(&apple_id) {
                                    eprintln!("Warning: Could not delete from macOS Contacts: {}", e);
                                }
                            }

                            if db.delete_person(person.id)? {
                                // Show undo prompt with 5 second timeout
                                if let Some(backup) = backup {
                                    if prompt_undo(&format!("Deleted \"{}\"", display_name), 5)? {
                                        db.restore_person(&backup)?;
                                        println!("Restored.");
                                        // Stay on same contact - don't advance index
                                    } else {
                                        // User didn't undo - advance if needed
                                        if index >= total - 1 && index > 0 {
                                            index -= 1;
                                        }
                                    }
                                } else {
                                    println!("Deleted: {}", display_name);
                                    if index >= total - 1 && index > 0 {
                                        index -= 1;
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Char('v') | KeyCode::Char('V') => {
                        view_mode = ViewMode::Table;
                        scroll = index.saturating_sub(visible_lines() / 2);
                    }
                    KeyCode::Down | KeyCode::Char('j') | KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') => {
                        if index + 1 < total {
                            index += 1;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::Left => {
                        index = index.saturating_sub(1);
                    }
                    KeyCode::Home | KeyCode::Char('g') => {
                        index = 0;
                    }
                    KeyCode::End | KeyCode::Char('G') => {
                        index = total - 1;
                    }
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('Q') => {
                        clear_screen()?;
                        std::process::exit(0);
                    }
                    KeyCode::Char('?') => {
                        show_help("browse")?;
                    }
                    _ => {}
                }
            }
            ViewMode::Table => {
                // Table view - show list with scrolling
                let visible = visible_lines();

                // Adjust scroll to keep cursor visible
                if index < scroll {
                    scroll = index;
                } else if index >= scroll + visible {
                    scroll = index - visible + 1;
                }

                // Fetch rows for display
                let rows = db.list_contact_rows(
                    visible as u32,
                    scroll as u32,
                    "sort_name",
                    "ASC",
                )?;

                print_table_header();
                for (i, row) in rows.iter().enumerate() {
                    let absolute_idx = scroll + i;
                    print_contact_row(row, absolute_idx == index);
                }

                let status = StatusBar::new()
                    .counter(index + 1, total)
                    .action("↑↓", "")
                    .action("g/G", "")
                    .action("enter", "")
                    .action("v", "iew")
                    .separator()
                    .action("?", "")
                    .action("q", "")
                    .action("Q", "uit")
                    .render();
                println!("\n{}", status);

                let code = {
                    let _guard = RawModeGuard::new()?;
                    match event::read()? {
                        Event::Key(KeyEvent { code, .. }) => code,
                        _ => continue,
                    }
                };

                match code {
                    KeyCode::Down | KeyCode::Char('j') => {
                        if index + 1 < total {
                            index += 1;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        index = index.saturating_sub(1);
                    }
                    KeyCode::PageDown | KeyCode::Char(' ') => {
                        index = (index + visible).min(total - 1);
                    }
                    KeyCode::PageUp => {
                        index = index.saturating_sub(visible);
                    }
                    KeyCode::Home | KeyCode::Char('g') => {
                        index = 0;
                    }
                    KeyCode::End | KeyCode::Char('G') => {
                        index = total - 1;
                    }
                    KeyCode::Enter => {
                        view_mode = ViewMode::Card;
                    }
                    KeyCode::Char('v') | KeyCode::Char('V') => {
                        view_mode = ViewMode::Card;
                    }
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('Q') => {
                        clear_screen()?;
                        std::process::exit(0);
                    }
                    KeyCode::Char('?') => {
                        show_help("list")?;
                    }
                    _ => {}
                }
            }
        }
    }

    clear_screen()?;
    Ok(())
}

// ============================================================================
// Edit Functions
// ============================================================================

/// Full edit: all fields including work, location, extended name fields
pub fn handle_full_edit(
    db: &Database,
    detail: &crate::models::ContactDetail,
) -> Result<bool> {
    // This is the renamed handle_edit_all - comprehensive field editing
    handle_edit_all(db, detail)
}

/// Handle edit all fields action - comprehensive field editing
pub fn handle_edit_all(
    db: &Database,
    detail: &crate::models::ContactDetail,
) -> Result<bool> {
    let person = &detail.person;

    // Helper macro to reduce repetition
    macro_rules! get_field {
        ($field:expr, $current:expr) => {
            match prompt_field($field, Some($current))? {
                FormResult::Value(v) => v,
                FormResult::Cancelled => {
                    println!("Cancelled.");
                    return Ok(false);
                }
            }
        };
    }

    // Name fields
    let current_first = person.name_given.as_deref().unwrap_or("");
    let new_first = get_field!("first", current_first);

    let current_last = person.name_family.as_deref().unwrap_or("");
    let new_last = get_field!("last", current_last);

    let current_middle = person.name_middle.as_deref().unwrap_or("");
    let new_middle = get_field!("middle", current_middle);

    let current_nickname = person.name_nickname.as_deref().unwrap_or("");
    let new_nickname = get_field!("nickname", current_nickname);

    let current_prefix = person.name_prefix.as_deref().unwrap_or("");
    let new_prefix = get_field!("prefix", current_prefix);

    let current_suffix = person.name_suffix.as_deref().unwrap_or("");
    let new_suffix = get_field!("suffix", current_suffix);

    let current_preferred = person.preferred_name.as_deref().unwrap_or("");
    let new_preferred = get_field!("preferred name", current_preferred);

    // Name order selection
    let name_order_options = ["Western (Given Family)", "Eastern (Family Given)", "Latin"];
    let current_order_idx = match person.name_order {
        NameOrder::Eastern => 1,
        NameOrder::Latin => 2,
        NameOrder::Western => 0,
    };
    println!("name order [{}]: ", name_order_options[current_order_idx]);
    let new_name_order = match select("", &name_order_options)? {
        Some(idx) => match idx {
            1 => NameOrder::Eastern,
            2 => NameOrder::Latin,
            _ => NameOrder::Western,
        },
        None => person.name_order, // Keep current on cancel
    };

    // Person type selection
    let person_type_options = ["Personal", "Business", "Prospect", "Connector"];
    let current_type_idx = match person.person_type {
        PersonType::Business => 1,
        PersonType::Prospect => 2,
        PersonType::Connector => 3,
        PersonType::Personal => 0,
    };
    println!("contact type [{}]: ", person_type_options[current_type_idx]);
    let new_person_type = match select("", &person_type_options)? {
        Some(idx) => match idx {
            1 => PersonType::Business,
            2 => PersonType::Prospect,
            3 => PersonType::Connector,
            _ => PersonType::Personal,
        },
        None => person.person_type, // Keep current on cancel
    };

    // Organization fields
    let (current_company, current_title, current_dept) = detail
        .organizations
        .first()
        .map(|(po, org)| {
            (
                org.name.as_str(),
                po.title.as_deref().unwrap_or(""),
                po.department.as_deref().unwrap_or(""),
            )
        })
        .unwrap_or(("", "", ""));

    let new_company = get_field!("company", current_company);
    let new_title = get_field!("title", current_title);
    let new_dept = get_field!("department", current_dept);

    // Emails - edit each one
    let mut new_emails: Vec<(uuid::Uuid, String, String)> = Vec::new();
    for email in &detail.emails {
        let type_label = email.email_type.as_str();
        let field_name = format!("email {}", type_label);
        let new_val = get_field!(&field_name, &email.email_address);

        if !new_val.is_empty() && new_val != email.email_address && !is_valid_email(&new_val) {
            eprintln!("Invalid email format");
            new_emails.push((email.id, email.email_address.clone(), type_label.to_string()));
        } else {
            new_emails.push((email.id, new_val, type_label.to_string()));
        }
    }

    // Phones - edit each one
    let mut new_phones: Vec<(uuid::Uuid, String, String)> = Vec::new();
    for phone in &detail.phones {
        let type_label = phone.phone_type.as_str();
        let field_name = format!("phone {}", type_label);
        let new_val = get_field!(&field_name, &phone.phone_number);
        new_phones.push((phone.id, new_val, type_label.to_string()));
    }

    // Addresses - edit each one
    struct AddressEdit {
        street: String,
        city: String,
        state: String,
        postal_code: String,
        country: String,
    }
    let mut new_addresses: Vec<AddressEdit> = Vec::new();
    for addr in &detail.addresses {
        let type_label = addr.address_type.as_str();
        println!("\n--- Address ({}) ---", type_label);

        let current_street = addr.street.as_deref().unwrap_or("");
        let new_street = get_field!("street", current_street);

        let current_city = addr.city.as_deref().unwrap_or("");
        let new_city = get_field!("city", current_city);

        let current_state = addr.state.as_deref().unwrap_or("");
        let new_state = get_field!("state", current_state);

        let current_postal = addr.postal_code.as_deref().unwrap_or("");
        let new_postal = get_field!("postal code", current_postal);

        let current_country = addr.country.as_deref().unwrap_or("");
        let new_country = get_field!("country", current_country);

        new_addresses.push(AddressEdit {
            street: new_street,
            city: new_city,
            state: new_state,
            postal_code: new_postal,
            country: new_country,
        });
    }

    // Notes
    let current_notes = person.notes.as_deref().unwrap_or("");
    let new_notes = get_field!("notes", current_notes);

    // Photo - show status and offer to change
    let has_photo = photo_utils::photo_exists(person.id);
    let photo_status = if has_photo { "has photo" } else { "no photo" };
    println!("photo [{}]: ", photo_status);

    let photo_options = if has_photo {
        vec!["Keep current", "Replace photo", "Clear photo"]
    } else {
        vec!["No photo", "Add photo"]
    };

    let mut photo_changed = false;
    if let Some(choice) = select("", &photo_options)? {
        if has_photo {
            match choice {
                1 => {
                    // Replace photo - open file picker
                    if let Some(path) = FileDialog::new()
                        .add_filter("Images", &["jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff"])
                        .set_title("Select photo for contact")
                        .pick_file()
                    {
                        match photo_utils::save_photo(person.id, &path) {
                            Ok(()) => {
                                println!("Photo updated.");
                                photo_changed = true;
                            }
                            Err(e) => eprintln!("Failed to save photo: {}", e),
                        }
                    }
                }
                2 => {
                    // Clear photo
                    photo_utils::delete_photo(person.id);
                    println!("Photo cleared.");
                    photo_changed = true;
                }
                _ => {} // Keep current
            }
        } else if choice == 1 {
            // Add photo - open file picker
            if let Some(path) = FileDialog::new()
                .add_filter("Images", &["jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff"])
                .set_title("Select photo for contact")
                .pick_file()
            {
                match photo_utils::save_photo(person.id, &path) {
                    Ok(()) => {
                        println!("Photo added.");
                        photo_changed = true;
                    }
                    Err(e) => eprintln!("Failed to save photo: {}", e),
                }
            }
        }
    }

    // AI contact permission toggle
    let ai_status = if person.ai_contact_allowed { "allowed" } else { "blocked" };
    println!("allow AI contact [{}]: ", ai_status);
    let ai_options = ["Allow", "Don't allow"];
    let new_ai_contact_allowed = match select("", &ai_options)? {
        Some(idx) => idx == 0,
        None => person.ai_contact_allowed, // Keep current on cancel
    };

    // Apply person changes
    let mut updated_person = person.clone();
    let mut has_changes = false;

    if new_first != current_first {
        updated_person.name_given = non_empty(new_first);
        has_changes = true;
    }
    if new_last != current_last {
        updated_person.name_family = non_empty(new_last);
        has_changes = true;
    }
    if new_middle != current_middle {
        updated_person.name_middle = non_empty(new_middle);
        has_changes = true;
    }
    if new_nickname != current_nickname {
        updated_person.name_nickname = non_empty(new_nickname);
        has_changes = true;
    }
    if new_prefix != current_prefix {
        updated_person.name_prefix = non_empty(new_prefix);
        has_changes = true;
    }
    if new_suffix != current_suffix {
        updated_person.name_suffix = non_empty(new_suffix);
        has_changes = true;
    }
    if new_preferred != current_preferred {
        updated_person.preferred_name = non_empty(new_preferred.to_string());
        has_changes = true;
    }
    if new_name_order != person.name_order {
        updated_person.name_order = new_name_order;
        has_changes = true;
    }
    if new_person_type != person.person_type {
        updated_person.person_type = new_person_type;
        has_changes = true;
    }
    if new_notes != current_notes {
        updated_person.notes = non_empty(new_notes);
        has_changes = true;
    }
    if new_ai_contact_allowed != person.ai_contact_allowed {
        updated_person.ai_contact_allowed = new_ai_contact_allowed;
        has_changes = true;
    }

    // Recompute display names if any name field or name_order changed
    let name_changed = updated_person.name_prefix != person.name_prefix
        || updated_person.name_given != person.name_given
        || updated_person.name_middle != person.name_middle
        || updated_person.name_family != person.name_family
        || updated_person.name_suffix != person.name_suffix
        || updated_person.name_nickname != person.name_nickname
        || updated_person.name_order != person.name_order;

    if name_changed {
        updated_person.compute_names();
    }

    // Update person record if changed
    if has_changes {
        db.update_person(&updated_person)?;
    }

    // Handle organization changes
    let org_changed = new_company != current_company
        || new_title != current_title
        || new_dept != current_dept;

    if org_changed {
        db.delete_person_organizations(person.id)?;

        if !new_company.is_empty() {
            let org = db.get_or_create_organization(&new_company)?;
            let mut po = PersonOrganization::new(person.id, org.id);
            po.title = non_empty(new_title);
            po.department = non_empty(new_dept);
            po.is_primary = true;
            db.insert_person_organization(&po)?;
        }
        has_changes = true;
    }

    // Handle email updates
    for (i, (_id, new_val, _type_str)) in new_emails.iter().enumerate() {
        let original = &detail.emails[i];
        if *new_val != original.email_address {
            let mut updated_email = original.clone();
            updated_email.email_address = new_val.clone();
            db.update_email(&updated_email)?;
            has_changes = true;
        }
    }

    // Handle phone updates
    for (i, (_id, new_val, _type_str)) in new_phones.iter().enumerate() {
        let original = &detail.phones[i];
        if *new_val != original.phone_number {
            let mut updated_phone = original.clone();
            updated_phone.phone_number = new_val.clone();
            db.update_phone(&updated_phone)?;
            has_changes = true;
        }
    }

    // Handle address updates
    for (i, new_addr) in new_addresses.iter().enumerate() {
        let original = &detail.addresses[i];
        let changed = new_addr.street != original.street.as_deref().unwrap_or("")
            || new_addr.city != original.city.as_deref().unwrap_or("")
            || new_addr.state != original.state.as_deref().unwrap_or("")
            || new_addr.postal_code != original.postal_code.as_deref().unwrap_or("")
            || new_addr.country != original.country.as_deref().unwrap_or("");

        if changed {
            let mut updated_addr = original.clone();
            updated_addr.street = non_empty(new_addr.street.clone());
            updated_addr.city = non_empty(new_addr.city.clone());
            updated_addr.state = non_empty(new_addr.state.clone());
            updated_addr.postal_code = non_empty(new_addr.postal_code.clone());
            updated_addr.country = non_empty(new_addr.country.clone());
            db.update_address(&updated_addr)?;
            has_changes = true;
        }
    }

    if has_changes || photo_changed {
        println!("\nSaved.");
    }

    Ok(true)
}

/// Convert empty string to None
fn non_empty(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Handle notes editing
pub fn handle_notes(
    db: &Database,
    detail: &crate::models::ContactDetail,
) -> Result<()> {
    let person = &detail.person;
    let current_notes = person.notes.as_deref().unwrap_or("");

    let new_notes = match prompt_field("notes", Some(current_notes))? {
        FormResult::Value(v) => v,
        FormResult::Cancelled => {
            println!("Cancelled.");
            return Ok(());
        }
    };

    if new_notes != current_notes {
        let mut updated = person.clone();
        updated.notes = if new_notes.is_empty() { None } else { Some(new_notes) };
        db.update_person(&updated)?;
        println!("\nSaved.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Address, Email, Person, Phone};

    fn setup_test_db() -> Database {
        let db = Database::open_memory().unwrap();

        let names = vec![
            ("John", "Smith"),
            ("Jane", "Doe"),
            ("Alice", "Johnson"),
            ("Bob", "Williams"),
            ("Charlie", "Brown"),
        ];

        for (first, last) in names {
            let mut p = Person::new();
            p.name_given = Some(first.to_string());
            p.name_family = Some(last.to_string());
            p.compute_names();
            db.insert_person(&p).unwrap();
        }

        db
    }

    #[test]
    fn test_list_contact_rows() {
        let db = setup_test_db();
        let rows = db.list_contact_rows(10, 0, "sort_name", "ASC").unwrap();
        assert_eq!(rows.len(), 5);
    }

    #[test]
    fn test_list_contact_rows_pagination() {
        let db = setup_test_db();
        let rows = db.list_contact_rows(2, 0, "sort_name", "ASC").unwrap();
        assert_eq!(rows.len(), 2);

        let rows2 = db.list_contact_rows(2, 2, "sort_name", "ASC").unwrap();
        assert_eq!(rows2.len(), 2);
    }

    #[test]
    fn test_list_contact_rows_with_details() {
        let db = Database::open_memory().unwrap();

        let mut p = Person::new();
        p.name_given = Some("Test".to_string());
        p.name_family = Some("User".to_string());
        p.compute_names();
        db.insert_person(&p).unwrap();

        let mut email = Email::new(p.id, "test@example.com".to_string());
        email.is_primary = true;
        db.insert_email(&email).unwrap();

        let mut phone = Phone::new(p.id, "555-1234".to_string());
        phone.is_primary = true;
        db.insert_phone(&phone).unwrap();

        let mut addr = Address::new(p.id);
        addr.city = Some("Austin".to_string());
        addr.state = Some("TX".to_string());
        addr.is_primary = true;
        db.insert_address(&addr).unwrap();

        let rows = db.list_contact_rows(10, 0, "sort_name", "ASC").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].display_name, "Test User");
        assert_eq!(rows[0].primary_email, Some("test@example.com".to_string()));
        assert_eq!(rows[0].primary_phone, Some("555-1234".to_string()));
        assert_eq!(rows[0].location, Some("Austin, TX".to_string()));
    }

    #[test]
    fn test_sort_field_from_str() {
        assert!(matches!(SortField::parse("name"), Some(SortField::Name)));
        assert!(matches!(
            SortField::parse("created"),
            Some(SortField::Created)
        ));
        assert!(matches!(
            SortField::parse("updated"),
            Some(SortField::Updated)
        ));
        assert!(SortField::parse("invalid").is_none());
    }

    #[test]
    fn test_sort_order_from_str() {
        assert!(matches!(SortOrder::parse("asc"), Some(SortOrder::Asc)));
        assert!(matches!(SortOrder::parse("desc"), Some(SortOrder::Desc)));
        assert!(SortOrder::parse("invalid").is_none());
    }
}
