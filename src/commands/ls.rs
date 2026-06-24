/// `wt ls` — list worktrees with status.
use anyhow::Result;
use owo_colors::OwoColorize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug)]
struct Row {
    name: String,
    path: PathBuf,
    branch: String,
    dirty: bool,
    ahead: u32,
    behind: u32,
}

/// Run the `ls` command.
pub fn run() -> Result<()> {
    let worktrees = crate::git::worktree_list()?;

    // Collect status concurrently.
    let rows: Arc<Mutex<Vec<Row>>> = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();

    for wt in worktrees {
        let rows = Arc::clone(&rows);
        let handle = thread::spawn(move || {
            let st = crate::git::status(&wt.path).unwrap_or(crate::git::WorktreeStatus {
                dirty: false,
                ahead: 0,
                behind: 0,
            });
            let name = wt
                .path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| wt.path.display().to_string());
            let branch = wt.branch.clone().unwrap_or_else(|| "(detached)".to_string());
            rows.lock().unwrap().push(Row {
                name,
                path: wt.path,
                branch,
                dirty: st.dirty,
                ahead: st.ahead,
                behind: st.behind,
            });
        });
        handles.push(handle);
    }

    for h in handles {
        let _ = h.join();
    }

    let mut rows = Arc::try_unwrap(rows).unwrap().into_inner().unwrap();
    // Sort by name for stable output.
    rows.sort_by(|a, b| a.name.cmp(&b.name));

    // Compute column widths.
    let name_w = rows.iter().map(|r| r.name.len()).max().unwrap_or(4).max(4);
    let branch_w = rows.iter().map(|r| r.branch.len()).max().unwrap_or(6).max(6);

    // Header
    println!(
        "{:<name_w$}  {:<branch_w$}  {}",
        "NAME".bold(),
        "BRANCH".bold(),
        "STATUS".bold(),
        name_w = name_w,
        branch_w = branch_w,
    );

    for row in &rows {
        let status_str = build_status(row);
        println!(
            "{:<name_w$}  {:<branch_w$}  {}",
            row.name.green(),
            row.branch.cyan(),
            status_str,
            name_w = name_w,
            branch_w = branch_w,
        );
        println!(
            "{:<name_w$}  {}",
            "",
            row.path.display().to_string().dimmed(),
            name_w = name_w,
        );
    }
    Ok(())
}

fn build_status(row: &Row) -> String {
    let mut parts = Vec::new();
    if row.dirty {
        parts.push("dirty".red().to_string());
    } else {
        parts.push("clean".green().to_string());
    }
    if row.ahead > 0 {
        parts.push(format!("●{} ahead", row.ahead).yellow().to_string());
    }
    if row.behind > 0 {
        parts.push(format!("●{} behind", row.behind).yellow().to_string());
    }
    parts.join("  ")
}
