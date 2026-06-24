/// Thin wrappers around the `git` binary.
/// All functions return `anyhow::Result`; stderr is captured and surfaced in errors.
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Run a git command, return stdout as a String (trimmed).
fn run_git(args: &[&str], cwd: Option<&Path>) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let out = cmd.output().context("failed to spawn git")?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        bail!("git {} failed: {}", args.join(" "), stderr)
    }
}

/// Return the absolute path of the top-level working tree (current worktree root).
pub fn toplevel() -> Result<PathBuf> {
    let s = run_git(&["rev-parse", "--show-toplevel"], None)?;
    Ok(PathBuf::from(s))
}

/// Return the absolute path of the *main* worktree (the one that owns .git).
/// Uses `git worktree list --porcelain`; the first entry is always the main worktree.
pub fn main_worktree() -> Result<PathBuf> {
    let worktrees = worktree_list()?;
    worktrees
        .into_iter()
        .next()
        .map(|w| w.path)
        .ok_or_else(|| anyhow::anyhow!("no worktrees found"))
}

/// Return the resolved path of the `.git` common dir.
#[allow(dead_code)]
pub fn common_dir() -> Result<PathBuf> {
    // `git rev-parse --git-common-dir` returns a path relative to cwd.
    let rel = run_git(&["rev-parse", "--git-common-dir"], None)?;
    let p = PathBuf::from(rel);
    Ok(if p.is_absolute() {
        p
    } else {
        std::env::current_dir()?.join(p)
    })
}

/// Resolve the default branch (e.g. "main" or "master") by checking
/// `origin/HEAD`, falling back to the current HEAD.
pub fn default_branch() -> Result<String> {
    // Try symbolic-ref for origin/HEAD first.
    if let Ok(s) = run_git(&["symbolic-ref", "refs/remotes/origin/HEAD"], None) {
        // s looks like "refs/remotes/origin/main"
        if let Some(branch) = s.strip_prefix("refs/remotes/origin/") {
            return Ok(branch.to_string());
        }
    }
    // Fall back to the current HEAD branch name.
    run_git(&["rev-parse", "--abbrev-ref", "HEAD"], None)
}

/// A parsed entry from `git worktree list --porcelain`.
#[derive(Debug, Clone)]
pub struct WorktreeEntry {
    pub path: PathBuf,
    pub branch: Option<String>, // None if detached
    #[allow(dead_code)]
    pub head: String,
}

/// List all worktrees.
pub fn worktree_list() -> Result<Vec<WorktreeEntry>> {
    let raw = run_git(&["worktree", "list", "--porcelain"], None)?;
    let mut entries = Vec::new();
    let mut path: Option<PathBuf> = None;
    let mut branch: Option<String> = None;
    let mut head: Option<String> = None;

    for line in raw.lines() {
        if line.starts_with("worktree ") {
            // Flush previous
            if let (Some(p), Some(h)) = (path.take(), head.take()) {
                entries.push(WorktreeEntry { path: p, branch: branch.take(), head: h });
            }
            path = Some(PathBuf::from(line.trim_start_matches("worktree ")));
        } else if let Some(rest) = line.strip_prefix("HEAD ") {
            head = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("branch ") {
            // "refs/heads/main" -> "main"
            let b = rest.strip_prefix("refs/heads/").unwrap_or(rest).to_string();
            branch = Some(b);
        } else if line == "detached" {
            branch = None;
        }
    }
    // Flush last
    if let (Some(p), Some(h)) = (path.take(), head.take()) {
        entries.push(WorktreeEntry { path: p, branch: branch.take(), head: h });
    }
    Ok(entries)
}

/// Status information for a worktree.
#[derive(Debug)]
pub struct WorktreeStatus {
    pub dirty: bool,
    pub ahead: u32,
    pub behind: u32,
}

/// Run `git status --porcelain=v2 --branch` in the given worktree path.
pub fn status(worktree_path: &Path) -> Result<WorktreeStatus> {
    let raw = run_git(
        &["status", "--porcelain=v2", "--branch"],
        Some(worktree_path),
    )?;
    let mut ahead = 0u32;
    let mut behind = 0u32;
    let mut dirty = false;

    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("# branch.ab ") {
            // Format: "+<ahead> -<behind>"
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() == 2 {
                ahead = parts[0].trim_start_matches('+').parse().unwrap_or(0);
                behind = parts[1].trim_start_matches('-').parse().unwrap_or(0);
            }
        } else if !line.starts_with('#') && !line.is_empty() {
            dirty = true;
        }
    }
    Ok(WorktreeStatus { dirty, ahead, behind })
}

/// Create a new worktree with a new branch.
pub fn worktree_add(branch: &str, path: &Path, from: &str) -> Result<()> {
    run_git(
        &[
            "worktree",
            "add",
            "-b",
            branch,
            path.to_str().unwrap(),
            from,
        ],
        None,
    )?;
    Ok(())
}

/// Remove a worktree.
pub fn worktree_remove(path: &Path, force: bool) -> Result<()> {
    let path_str = path.to_str().unwrap();
    if force {
        run_git(&["worktree", "remove", "--force", path_str], None)?;
    } else {
        run_git(&["worktree", "remove", path_str], None)?;
    }
    Ok(())
}

/// Delete a branch.
pub fn branch_delete(branch: &str) -> Result<()> {
    run_git(&["branch", "-D", branch], None)?;
    Ok(())
}

/// Run `git worktree prune` and return its output.
pub fn prune() -> Result<String> {
    // prune writes to stderr; we capture both.
    let mut cmd = Command::new("git");
    cmd.args(["worktree", "prune", "--verbose"]);
    let out = cmd.output().context("failed to spawn git")?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if !out.status.success() {
        bail!("git worktree prune failed: {}", stderr);
    }
    Ok(if stdout.is_empty() { stderr } else { stdout })
}
