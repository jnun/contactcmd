use clap::Parser;
use contactcmd::cli::{run_add, run_browse_mode, run_list, run_menu, run_messages, run_photo, run_search, run_show, run_sync, Cli, Commands};
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
        Some(Commands::Browse(args)) => {
            let persons = if args.missing_email {
                db.find_persons_missing_email(10000)?
            } else if args.missing_phone {
                db.find_persons_missing_phone(10000)?
            } else if let Some(ref query) = args.search {
                let words: Vec<&str> = query.split_whitespace().collect();
                db.search_persons_multi(&words, false, 10000)?
            } else {
                db.list_persons(10000, 0)?
            };
            run_browse_mode(&db, persons)?;
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
        Some(Commands::Sync(args)) => {
            run_sync(&db, &args.source, args.dry_run)?;
        }
        Some(Commands::Messages(args)) => {
            run_messages(&db, &args.query, args.since.as_deref())?;
        }
        Some(Commands::Photo(args)) => {
            run_photo(&db, &args.identifier, args.path.as_deref(), args.clear)?;
        }
    }

    Ok(())
}
