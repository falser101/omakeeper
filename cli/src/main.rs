mod clean;
mod history;
mod installer;
mod purge;
mod ui;
mod util;
mod whitelist;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "omakeeper",
    version,
    about = "System maintenance for Omarchy — clean, purge, installers, and more",
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
    /// Live system status (planned)
    Status,
    /// Disk analyzer (planned)
    Analyze,
    /// Package uninstall helper (planned)
    Uninstall,
    /// Bounded system maintenance (planned)
    Optimize,
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
        }) => {
            if whitelist {
                whitelist::list(cli.json)
            } else {
                clean::run(dry_run, yes, cli.json)
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
        Some(Commands::Status)
        | Some(Commands::Analyze)
        | Some(Commands::Uninstall)
        | Some(Commands::Optimize) => {
            eprintln!("This command is planned for a later phase. See README.");
            std::process::exit(2);
        }
    }
}
