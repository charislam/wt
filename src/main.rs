mod commands;
mod config;
mod git;
mod setup;

use anyhow::Result;
use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;

#[derive(Parser)]
#[command(name = "wt", about = "Git worktree manager", version)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Write a .worktree.toml template to the repo root.
    Init {
        /// Overwrite an existing .worktree.toml.
        #[arg(long)]
        force: bool,
    },
    /// Create a new worktree.
    New {
        /// Name for the new worktree (and default branch name).
        name: String,
        /// Base ref to branch from (default: repo default branch).
        #[arg(long)]
        from: Option<String>,
        /// Override the branch name (defaults to <name>).
        #[arg(long, value_name = "BRANCH")]
        branch: Option<String>,
        /// Skip running setup commands.
        #[arg(long)]
        no_setup: bool,
        /// Skip copying files.
        #[arg(long)]
        no_copy: bool,
        /// Print only the final worktree path to stdout (for shell integration).
        #[arg(long)]
        print_path: bool,
    },
    /// List all worktrees with status.
    Ls,
    /// Print the absolute path of a worktree.
    Path {
        /// Worktree name (last path component).
        name: String,
    },
    /// Change directory to a worktree (shell function consumes this).
    Cd {
        /// Worktree name (last path component).
        name: String,
    },
    /// Re-run copy + setup steps for an existing worktree.
    Setup {
        /// Worktree name.
        name: String,
    },
    /// Remove a worktree.
    Rm {
        /// Worktree name.
        name: String,
        /// Allow removal of a dirty worktree.
        #[arg(long)]
        force: bool,
        /// Also delete the worktree's branch.
        #[arg(long)]
        delete_branch: bool,
    },
    /// Run `git worktree prune`.
    Prune,
    /// Print a shell integration function.
    Shellinit {
        /// Target shell (default: zsh).
        #[arg(value_enum, default_value_t = commands::shellinit::Shell::Zsh)]
        shell: commands::shellinit::Shell,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("{} {}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Cmd::Init { force } => commands::init::run(force),
        Cmd::New {
            name,
            from,
            branch,
            no_setup,
            no_copy,
            print_path,
        } => commands::new::run(
            &name,
            from.as_deref(),
            branch.as_deref(),
            no_setup,
            no_copy,
            print_path,
        ),
        Cmd::Ls => commands::ls::run(),
        Cmd::Path { name } => commands::path::run(&name),
        Cmd::Cd { name } => commands::path::run(&name),
        Cmd::Setup { name } => commands::setup::run(&name),
        Cmd::Rm {
            name,
            force,
            delete_branch,
        } => commands::rm::run(&name, force, delete_branch),
        Cmd::Prune => commands::prune::run(),
        Cmd::Shellinit { shell } => commands::shellinit::run(&shell),
    }
}
