mod analyze;
mod clean;
mod etch;
mod history;
mod installer;
mod optimize;
mod purge;
mod status;
mod ui;
mod uninstall;
mod util;
mod whitelist;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "omakeeper",
    version,
    about = "System maintenance for Omarchy — clean, purge, uninstall, optimize, analyze, and status",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Emit machine-readable JSON where supported
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Scan and remove safe-to-regenerate caches and junk
    Clean {
        /// Preview only; do not delete
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation prompts (use after reviewing dry-run)
        #[arg(long)]
        yes: bool,
        /// Manage protected paths instead of cleaning
        #[arg(long)]
        whitelist: bool,
        /// JSON array of item ids or paths to select
        #[arg(long, value_name = "FILE")]
        select_file: Option<String>,
    },
    /// Find and remove rebuildable project artifacts
    Purge {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        yes: bool,
        /// Edit or list custom scan roots
        #[arg(long)]
        paths: bool,
    },
    /// Find leftover installer files (iso, AppImage, pkg archives, …)
    Installer {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        yes: bool,
    },
    /// Show recent cleanup operations
    History {
        /// Number of entries to show
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Manage clean whitelist
    Whitelist {
        #[command(subcommand)]
        action: Option<WhitelistCmd>,
    },
    /// Interactive main menu (default when no subcommand)
    Menu,
    /// Live system status dashboard
    Status {
        /// Stream snapshots until quit
        #[arg(long)]
        watch: bool,
        /// Watch interval in seconds
        #[arg(long, default_value_t = 2)]
        interval: u64,
    },
    /// Disk explorer
    #[command(visible_alias = "analyse")]
    Analyze {
        /// Directory to open (default: home overview)
        #[arg(value_name = "PATH")]
        path: Option<String>,
    },
    /// Remove packages and leftover user data
    Uninstall {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        yes: bool,
        /// User leftover dirs to delete (opt-in; omitted dirs are kept)
        #[arg(long)]
        leftover: Vec<String>,
        /// Package names (omit to pick interactively)
        packages: Vec<String>,
    },
    /// Bounded system maintenance
    Optimize {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        yes: bool,
        /// Edit skipped optimize tasks
        #[arg(long)]
        whitelist: bool,
        /// JSON array of task ids to select
        #[arg(long, value_name = "FILE")]
        select_file: Option<String>,
    },
    /// Move paths to the XDG trash
    Trash {
        paths: Vec<String>,
    },
    /// Stream OMARCHY wordmark etch frames from ttfx as JSONL
    Etch {
        /// Effect name, or random (default)
        #[arg(long, default_value = "random")]
        effect: String,
    },
}

#[derive(Subcommand, Debug)]
enum WhitelistCmd {
    List,
    Add { path: String },
    Remove { path: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None | Some(Commands::Menu) => ui::menu::run(cli.json),
        Some(Commands::Clean {
            dry_run,
            yes,
            whitelist,
            select_file,
        }) => {
            if whitelist {
                whitelist::list(cli.json)
            } else {
                clean::run(dry_run, yes, cli.json, select_file.as_deref())
            }
        }
        Some(Commands::Purge {
            dry_run,
            yes,
            paths,
        }) => {
            if paths {
                purge::manage_paths(cli.json)
            } else {
                purge::run(dry_run, yes, cli.json)
            }
        }
        Some(Commands::Installer { dry_run, yes }) => installer::run(dry_run, yes, cli.json),
        Some(Commands::History { limit }) => history::show(limit, cli.json),
        Some(Commands::Whitelist { action }) => match action.unwrap_or(WhitelistCmd::List) {
            WhitelistCmd::List => whitelist::list(cli.json),
            WhitelistCmd::Add { path } => whitelist::add(&path),
            WhitelistCmd::Remove { path } => whitelist::remove(&path),
        },
        Some(Commands::Status { watch, interval }) => status::run(watch, interval, cli.json),
        Some(Commands::Analyze { path }) => analyze::run(path, cli.json),
        Some(Commands::Uninstall {
            dry_run,
            yes,
            leftover,
            packages,
        }) => uninstall::run(dry_run, yes, cli.json, &packages, &leftover),
        Some(Commands::Optimize {
            dry_run,
            yes,
            whitelist,
            select_file,
        }) => {
            if whitelist {
                optimize::manage_whitelist(cli.json)
            } else {
                optimize::run(dry_run, yes, cli.json, select_file.as_deref())
            }
        }
        Some(Commands::Trash { paths }) => trash_paths(&paths, cli.json),
        Some(Commands::Etch { effect }) => etch::run(Some(effect.as_str())),
    }
}

fn trash_paths(paths: &[String], json: bool) -> Result<()> {
    let mut moved = Vec::new();
    let mut errors = Vec::new();
    for raw in paths {
        let path = util::expand_user(raw);
        match util::move_to_trash(&path) {
            Ok(()) => moved.push(path.display().to_string()),
            Err(e) => errors.push(format!("{}: {e}", path.display())),
        }
    }
    if json {
        println!(
            "{}",
            serde_json::json!({ "moved": moved, "errors": errors })
        );
    } else {
        for p in &moved {
            println!("  ✓ {p}");
        }
        for e in &errors {
            eprintln!("  ✗ {e}");
        }
    }
    if !errors.is_empty() {
        anyhow::bail!("failed to trash {} path(s)", errors.len());
    }
    Ok(())
}
