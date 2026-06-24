/// `wt init` — write a `.worktree.toml` template to the repo root.
use anyhow::{bail, Result};

const TEMPLATE: &str = r#"copy = [".env", ".env.local"]
setup = ["pnpm install"]

[worktrees]
dir = "~/worktrees/{repo}/{name}"
branch_prefix = ""
"#;

/// Run the `init` command.
pub fn run(force: bool) -> Result<()> {
    let root = crate::git::toplevel()?;
    let dest = root.join(".worktree.toml");
    if dest.exists() && !force {
        bail!(
            "{} already exists. Use --force to overwrite.",
            dest.display()
        );
    }
    std::fs::write(&dest, TEMPLATE)?;
    eprintln!("wrote {}", dest.display());
    Ok(())
}
