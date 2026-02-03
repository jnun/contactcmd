use clap::{Args, Parser, Subcommand};

pub mod add;
pub mod ai;
pub mod bridge;
pub mod chat;
pub mod checkin;
pub mod cleanup;
pub mod display;
pub mod email;
pub mod gateway;
pub mod google_auth;
pub mod import;
pub mod learn;
pub mod list;
pub mod menu;
pub mod messages;
pub mod photo;
pub mod photo_utils;
pub mod search;
pub mod setup;
pub mod show;
pub mod sync;
pub mod task;
pub mod ui;

pub use add::run_add;
pub use bridge::run_bridge;
pub use checkin::run_checkin;
pub use cleanup::run_cleanup;
pub use display::print_full_contact;
pub use gateway::run_gateway;
pub use import::{pick_csv_file, run_import};
pub use learn::{run_learn, run_learn_progress};
pub use list::{run_browse, run_list, ViewMode};
pub use menu::run_menu;
pub use messages::run_messages;
pub use photo::run_photo;
pub use search::run_search;
pub use setup::run_setup;
pub use show::run_show;
pub use sync::run_sync;
pub use task::run_tasks;

#[derive(Parser)]
#[command(name = "contactcmd")]
#[command(about = "Personal CRM for the command line")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Direct search query (e.g., `contactcmd john`)
    #[arg(trailing_var_arg = true)]
    pub query: Vec<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List contacts with pagination
    List(ListArgs),
    /// Browse contacts one by one with full details
    Browse(BrowseArgs),
    /// Search contacts by name or email
    Search(SearchArgs),
    /// Show full details for a contact
    Show(ShowArgs),
    /// Add a new contact
    Add(AddArgs),
    /// Import contacts from a CSV file
    Import(ImportArgs),
    /// Sync with external sources
    Sync(SyncArgs),
    /// Search iMessage history
    Messages(MessagesArgs),
    /// Set or clear a contact's photo
    Photo(PhotoArgs),
    /// Manage tags for contacts
    Tag(TagArgs),
    /// Moltbot bridge for iMessage/SMS integration
    Bridge(bridge::BridgeArgs),
    /// Communication gateway for AI agent message approval
    Gateway(gateway::GatewayArgs),
    /// Manage follow-up checkins for contacts
    Checkin(CheckinArgs),
    /// Learn about app features
    Learn(LearnArgs),
}

#[derive(Args)]
pub struct ListArgs {
    #[arg(short, long, default_value = "1")]
    pub page: u32,
    #[arg(short, long, default_value = "20")]
    pub limit: u32,
    #[arg(short, long)]
    pub sort: Option<String>,
    #[arg(short, long, default_value = "asc")]
    pub order: String,
    #[arg(short, long)]
    pub all: bool,
    /// Review contacts one by one with edit/delete options
    #[arg(short, long)]
    pub review: bool,
}

#[derive(Args)]
pub struct SearchArgs {
    /// Search query (name, email, notes, etc.)
    #[arg(default_value = "")]
    pub query: String,
    #[arg(short, long)]
    pub case_sensitive: bool,
    /// Find contacts missing info: "phone", "email", or "contact"
    #[arg(short, long, value_name = "FIELD")]
    pub missing: Option<String>,
    /// Limit search to specific field: "name", "city", "state", or "note"
    #[arg(short, long, value_name = "FIELD")]
    pub field: Option<String>,
}

#[derive(Args)]
pub struct ShowArgs {
    pub identifier: String,
}

#[derive(Args)]
pub struct AddArgs {
    #[arg(short, long)]
    pub first: Option<String>,
    #[arg(short, long)]
    pub last: Option<String>,
    #[arg(short, long)]
    pub email: Option<String>,
    #[arg(short, long)]
    pub phone: Option<String>,
    #[arg(short, long)]
    pub company: Option<String>,
    #[arg(short, long)]
    pub title: Option<String>,
    #[arg(short, long)]
    pub notes: Option<String>,
}

#[derive(Args)]
pub struct ImportArgs {
    /// Path to CSV file to import (opens file picker if omitted)
    pub file: Option<String>,
    /// Preview import without making changes
    #[arg(long)]
    pub dry_run: bool,
    /// Label to track import origin (e.g., "linkedin", "conference2024")
    #[arg(long)]
    pub source: Option<String>,
}

#[derive(Args)]
pub struct MessagesArgs {
    /// Search query for messages
    pub query: String,
    /// Only show messages since this date (YYYY-MM-DD)
    #[arg(long)]
    pub since: Option<String>,
}

#[derive(Args)]
pub struct SyncArgs {
    pub source: String,
    #[arg(short, long)]
    pub dry_run: bool,
}

#[derive(Args)]
pub struct PhotoArgs {
    /// Contact name or UUID
    pub identifier: String,
    /// Path to image file (jpg, png, gif, webp)
    pub path: Option<String>,
    /// Clear existing photo
    #[arg(short, long)]
    pub clear: bool,
}

#[derive(Args)]
pub struct BrowseArgs {
    /// Browse only contacts missing an email address
    #[arg(long)]
    pub missing_email: bool,
    /// Browse only contacts missing a phone number
    #[arg(long)]
    pub missing_phone: bool,
    /// Browse contacts matching a search term
    #[arg(short, long)]
    pub search: Option<String>,
    /// Browse contacts with a specific tag (prompts for selection if no value given)
    #[arg(short, long)]
    pub tag: Option<Option<String>>,
}

#[derive(Args)]
pub struct TagArgs {
    #[command(subcommand)]
    pub command: TagCommands,
}

#[derive(Subcommand)]
pub enum TagCommands {
    /// List all tags
    List,
    /// Tag all business-type contacts (from imports)
    TagBusiness {
        /// Tag name to apply
        tag: String,
    },
    /// Show contacts with a specific tag
    Show {
        /// Tag name
        tag: String,
    },
    /// Delete all contacts with a specific tag
    DeleteTagged {
        /// Tag name
        tag: String,
    },
    /// Remove a tag (keeps contacts)
    Remove {
        /// Tag name to remove
        tag: String,
    },
}

#[derive(Args)]
pub struct CheckinArgs {
    #[command(subcommand)]
    pub command: CheckinCommands,
}

#[derive(Subcommand)]
pub enum CheckinCommands {
    /// Show all due checkins (today + overdue)
    List,
    /// Show all scheduled checkins (including future)
    All,
    /// Set a checkin date for a contact
    Set {
        /// Contact name or UUID
        identifier: String,
        /// Date (YYYY-MM-DD, "today", "tomorrow", "+3d", "+1w")
        date: String,
    },
    /// Clear a contact's checkin (mark as done)
    Done {
        /// Contact name or UUID
        identifier: String,
    },
}

#[derive(Args)]
pub struct LearnArgs {
    /// Feature to learn about (shows next unlearned if omitted)
    pub query: Option<String>,
    /// Show learning progress instead of a tutorial
    #[arg(long)]
    pub progress: bool,
}
