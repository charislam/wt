/// `wt setup <name>` — re-run copy + setup steps for an existing worktree.
use anyhow::{bail, Result};

/// Run the `setup` command.
pub fn run(name: &str) -> Result<()> {
    let worktrees = crate::git::worktree_list()?;
    let entry = worktrees
        .iter()
        .find(|wt| {
            wt.path
                .file_name()
                .map(|s| s.to_string_lossy() == name)
                .unwrap_or(false)
        })
        .ok_or_else(|| anyhow::anyhow!("worktree '{}' not found", name))?;

    let main = crate::git::main_worktree()?;
    let cfg = crate::config::load(&main)?;

    if cfg.copy.is_empty() && cfg.setup.is_empty() {
        bail!("no copy or setup commands configured");
    }

    if !cfg.copy.is_empty() {
        crate::setup::copy_files(&cfg.copy, &main, &entry.path)?;
    }
    if !cfg.setup.is_empty() {
        crate::setup::run_setup_commands(&cfg.setup, &entry.path)?;
    }
    Ok(())
}
