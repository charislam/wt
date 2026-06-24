/// `wt rm <name>` — remove a worktree.
use anyhow::{bail, Context, Result};

/// Run the `rm` command.
pub fn run(name: &str, force: bool, delete_branch: bool) -> Result<()> {
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

    // Check dirty unless --force
    if !force {
        let st = crate::git::status(&entry.path)?;
        if st.dirty {
            bail!(
                "worktree '{}' has uncommitted changes. Use --force to remove anyway.",
                name
            );
        }
    }

    let branch = entry.branch.clone();
    let path = entry.path.clone();

    crate::git::worktree_remove(&path, force)
        .with_context(|| format!("removing worktree '{}'", name))?;
    eprintln!("removed worktree: {}", path.display());

    if delete_branch {
        if let Some(b) = branch {
            crate::git::branch_delete(&b)
                .with_context(|| format!("deleting branch '{}'", b))?;
            eprintln!("deleted branch: {}", b);
        } else {
            eprintln!("warning: worktree was in detached HEAD state; no branch to delete");
        }
    }
    Ok(())
}
