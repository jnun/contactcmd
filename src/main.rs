use clap::Parser;
use contactcmd::cli::{run_add, run_delete, run_list, run_menu, run_messages, run_note, run_search, run_show, run_sync, run_update, Cli, Commands};
use contactcmd::db::Database;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let db = Database::open()?;

    match cli.command {
        None => {
            // No subcommand provided - show interactive menu
            run_menu(&db)?;
        }
        Some(Commands::List(args)) => {
            run_list(&db, args.page, args.limit, args.sort, args.order, args.all, args.review)?;
        }
        Some(Commands::Search(args)) => {
            run_search(&db, &args.query, args.case_sensitive, args.missing.as_deref())?;
        }
        Some(Commands::Show(args)) => {
            run_show(&db, &args.identifier)?;
        }
        Some(Commands::Add(args)) => {
            run_add(&db, args.first, args.last, args.email, args.phone, args.notes)?;
        }
        Some(Commands::Update(args)) => {
            run_update(&db, &args.identifier, args.first, args.last, args.email, args.phone, args.notes)?;
        }
        Some(Commands::Delete(args)) => {
            run_delete(&db, &args.identifier, args.force)?;
        }
        Some(Commands::Note(args)) => {
            let note_text = if args.note.is_empty() {
                None
            } else {
                Some(args.note.join(" "))
            };
            run_note(&db, &args.search, note_text)?;
        }
        Some(Commands::Sync(args)) => {
            run_sync(&db, &args.source, args.dry_run)?;
        }
        Some(Commands::Messages(args)) => {
            run_messages(&db, &args.query)?;
        }
    }

    Ok(())
}
