/// `wt prune` — run `git worktree prune` and report output.
use anyhow::Result;

/// Run the `prune` command.
pub fn run() -> Result<()> {
    let out = crate::git::prune()?;
    if out.is_empty() {
        eprintln!("nothing to prune");
    } else {
        eprintln!("{}", out);
    }
    Ok(())
}
