use clap::{Args, Parser, Subcommand};

pub mod add;
pub mod cleanup;
pub mod display;
pub mod list;
pub mod menu;
pub mod messages;
pub mod photo;
pub mod photo_utils;
pub mod search;
pub mod show;
pub mod sync;
pub mod ui;

pub use add::run_add;
pub use cleanup::run_cleanup;
pub use display::print_full_contact;
pub use list::{run_browse_mode, run_list};
pub use menu::run_menu;
pub use messages::run_messages;
pub use photo::run_photo;
pub use search::run_search;
pub use show::run_show;
pub use sync::run_sync;

#[derive(Parser)]
#[command(name = "contactcmd")]
#[command(about = "Personal CRM for the command line")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
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
    /// Sync with external sources
    Sync(SyncArgs),
    /// Search iMessage history
    Messages(MessagesArgs),
    /// Set or clear a contact's photo
    Photo(PhotoArgs),
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
}
