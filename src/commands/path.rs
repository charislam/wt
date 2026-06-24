/// `wt path <name>` and `wt cd <name>` — print the absolute path of a worktree.
use anyhow::{bail, Result};

/// Find a worktree by name (last path component) and print its absolute path.
pub fn run(name: &str) -> Result<()> {
    let worktrees = crate::git::worktree_list()?;
    for wt in worktrees {
        let wt_name = wt
            .path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if wt_name == name {
            println!("{}", wt.path.display());
            return Ok(());
        }
    }
    bail!("worktree '{}' not found", name);
}
