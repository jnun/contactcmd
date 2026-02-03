use clap::Parser;
use contactcmd::cli::{pick_csv_file, run_add, run_bridge, run_browse, run_checkin, run_gateway, run_import, run_learn, run_learn_progress, run_list, run_menu, run_messages, run_photo, run_search, run_show, run_sync, Cli, Commands, TagCommands, ViewMode};
use contactcmd::db::Database;
use contactcmd::models::PersonType;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let db = Database::open()?;

    match cli.command {
        None => {
            if !cli.query.is_empty() {
                // Direct search mode: `contactcmd john` or `contactcmd john smith`
                let query = cli.query.join(" ");
                run_search(&db, &query, false, None, None)?;
            } else {
                // No args - show interactive menu
                run_menu(&db)?;
            }
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
            } else if let Some(tag_opt) = args.tag {
                // --tag was specified
                let tag_name = match tag_opt {
                    Some(name) => name,
                    None => {
                        // No value given, prompt for selection
                        let tags = db.list_tags()?;
                        if tags.is_empty() {
                            anyhow::bail!("No tags defined. Use 'contactcmd tag tag-business <name>' to create one.");
                        }
                        let options: Vec<String> = tags.iter().map(|t| {
                            let count = db.get_persons_by_tag(&t.name).map(|p| p.len()).unwrap_or(0);
                            format!("{} ({} contacts)", t.name, count)
                        }).collect();
                        let selection = inquire::Select::new("Select a tag:", options)
                            .without_filtering()
                            .prompt()?;
                        // Extract tag name (before the " (")
                        selection.split(" (").next().unwrap_or(&selection).to_string()
                    }
                };
                db.get_persons_by_tag(&tag_name)?
            } else {
                db.list_persons(10000, 0)?
            };
            run_browse(&db, persons, ViewMode::Card)?;
        }
        Some(Commands::Search(args)) => {
            run_search(&db, &args.query, args.case_sensitive, args.missing.as_deref(), args.field.as_deref())?;
        }
        Some(Commands::Show(args)) => {
            run_show(&db, &args.identifier)?;
        }
        Some(Commands::Add(args)) => {
            run_add(&db, args.first, args.last, args.email, args.phone, args.notes)?;
        }
        Some(Commands::Import(args)) => {
            let file = match args.file {
                Some(f) => f,
                None => pick_csv_file().ok_or_else(|| anyhow::anyhow!("No file selected"))?,
            };
            run_import(&db, &file, args.dry_run, args.source.as_deref())?;
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
        Some(Commands::Tag(args)) => {
            match args.command {
                TagCommands::List => {
                    let tags = db.list_tags()?;
                    if tags.is_empty() {
                        println!("No tags defined.");
                    } else {
                        println!("Tags:");
                        for tag in tags {
                            let count = db.get_persons_by_tag(&tag.name)?.len();
                            println!("  {} ({} contacts)", tag.name, count);
                        }
                    }
                }
                TagCommands::TagBusiness { tag } => {
                    let count = db.tag_persons_by_type(PersonType::Business, &tag)?;
                    println!("Tagged {} business contacts with '{}'", count, tag);
                }
                TagCommands::Show { tag } => {
                    let persons = db.get_persons_by_tag(&tag)?;
                    if persons.is_empty() {
                        println!("No contacts with tag '{}'", tag);
                    } else {
                        println!("Contacts with tag '{}' ({}):", tag, persons.len());
                        for person in persons.iter().take(20) {
                            println!("  {}", person.display_name.as_deref().unwrap_or("(unnamed)"));
                        }
                        if persons.len() > 20 {
                            println!("  ... and {} more", persons.len() - 20);
                        }
                    }
                }
                TagCommands::DeleteTagged { tag } => {
                    let count = db.get_persons_by_tag(&tag)?.len();
                    if count == 0 {
                        println!("No contacts with tag '{}'", tag);
                    } else {
                        println!("This will DELETE {} contacts with tag '{}'. Type 'yes' to confirm:", count, tag);
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input)?;
                        if input.trim() == "yes" {
                            let deleted = db.delete_persons_by_tag(&tag)?;
                            println!("Deleted {} contacts.", deleted);
                        } else {
                            println!("Cancelled.");
                        }
                    }
                }
                TagCommands::Remove { tag } => {
                    if db.delete_tag(&tag)? {
                        println!("Removed tag '{}' (contacts kept)", tag);
                    } else {
                        println!("Tag '{}' not found", tag);
                    }
                }
            }
        }
        Some(Commands::Bridge(args)) => {
            run_bridge(&db, args)?;
        }
        Some(Commands::Gateway(args)) => {
            run_gateway(&db, args)?;
        }
        Some(Commands::Checkin(args)) => {
            run_checkin(&db, args.command)?;
        }
        Some(Commands::Learn(args)) => {
            if args.progress {
                run_learn_progress(&db)?;
            } else {
                run_learn(&db, args.query.as_deref())?;
            }
        }
    }

    Ok(())
}
