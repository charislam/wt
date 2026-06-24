/// Shared logic for copying files and running setup commands inside a new worktree.
use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};

/// Copy each file in `files` from `src_root` into `dest_root`.
/// Missing files are skipped with a warning printed to stderr.
pub fn copy_files(files: &[String], src_root: &Path, dest_root: &Path) -> Result<()> {
    for rel in files {
        let src = src_root.join(rel);
        if !src.exists() {
            eprintln!("warning: copy source not found, skipping: {}", src.display());
            continue;
        }
        let dest = dest_root.join(rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating dir {}", parent.display()))?;
        }
        std::fs::copy(&src, &dest)
            .with_context(|| format!("copying {} -> {}", src.display(), dest.display()))?;
        eprintln!("copied {}", rel);
    }
    Ok(())
}

/// Run each command in `commands` sequentially with `cwd` as the working directory.
/// Output is streamed to stdout/stderr. Returns an error immediately on non-zero exit.
pub fn run_setup_commands(commands: &[String], cwd: &Path) -> Result<()> {
    for cmd_str in commands {
        eprintln!("$ {}", cmd_str);
        // Split simple commands via shell (sh -c) to support args.
        let status = Command::new("sh")
            .arg("-c")
            .arg(cmd_str)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .with_context(|| format!("failed to spawn: {}", cmd_str))?;

        if !status.success() {
            anyhow::bail!(
                "setup command failed (exit {}): {}",
                status.code().unwrap_or(-1),
                cmd_str
            );
        }
    }
    Ok(())
}
