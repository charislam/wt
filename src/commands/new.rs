/// `wt new <name>` — create a new git worktree.
use anyhow::{Context, Result};

/// Run the `new` command.
#[allow(clippy::too_many_arguments)]
pub fn run(
    name: &str,
    from: Option<&str>,
    branch_override: Option<&str>,
    no_setup: bool,
    no_copy: bool,
    print_path: bool,
) -> Result<()> {
    let main = crate::git::main_worktree()?;
    let cfg = crate::config::load(&main)?;

    // Resolve branch name
    let prefix = cfg.worktrees.branch_prefix.as_deref().unwrap_or("");
    let branch = branch_override
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{}{}", prefix, name));

    // Resolve the base ref
    let base = match from {
        Some(f) => f.to_string(),
        None => crate::git::default_branch()
            .unwrap_or_else(|_| "HEAD".to_string()),
    };

    // Resolve target directory
    let target = crate::config::resolve_dir(&cfg, &main, name)?;

    if !print_path {
        eprintln!(
            "creating worktree '{}' at {} (branch: {}, from: {})",
            name,
            target.display(),
            branch,
            base
        );
    }

    crate::git::worktree_add(&branch, &target, &base)
        .with_context(|| format!("git worktree add failed for '{}'", name))?;

    if !no_copy && !cfg.copy.is_empty() {
        crate::setup::copy_files(&cfg.copy, &main, &target)?;
    }

    if !no_setup && !cfg.setup.is_empty() {
        crate::setup::run_setup_commands(&cfg.setup, &target)?;
    }

    if print_path {
        println!("{}", target.display());
    } else {
        eprintln!("worktree ready: {}", target.display());
    }

    Ok(())
}
