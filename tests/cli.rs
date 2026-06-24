//! Integration tests for the `wt` binary.
//!
//! These tests run the compiled binary against throwaway git repos created in
//! temp directories. They NEVER touch the real `~/worktrees` — every test's
//! `.worktree.toml` points `worktrees.dir` at an absolute path inside a tempdir.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use tempfile::TempDir;

/// A throwaway repo plus the directory where its worktrees will be placed.
struct Repo {
    /// Keeps the tempdir alive for the duration of the test.
    _tmp: TempDir,
    /// The git repo working directory.
    repo: PathBuf,
    /// The directory worktrees get created under (sibling of repo).
    wt_out: PathBuf,
}

/// Run a git command in `cwd` and assert success.
fn git(cwd: &Path, args: &[&str]) {
    let status = StdCommand::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .expect("failed to spawn git");
    assert!(status.success(), "git {:?} failed in {}", args, cwd.display());
}

/// Build a `wt` command rooted at the repo dir.
fn wt(repo: &Path) -> Command {
    let mut cmd = Command::cargo_bin("wt").expect("binary built");
    cmd.current_dir(repo);
    cmd
}

/// Create a throwaway repo:
/// - tempdir with `repo/` (git working dir) and `out/` (worktree target) siblings
/// - `git init -b main`, configured user
/// - `.env`, `.gitignore`, `README.md`, initial commit
/// - `.worktree.toml` with `worktrees.dir` pointing at an absolute tempdir path
fn setup_repo_with(copy: &[&str], setup: &[&str]) -> Repo {
    let tmp = tempfile::tempdir().expect("tempdir");
    let base = tmp.path();
    let repo = base.join("repo");
    let wt_out = base.join("out");
    fs::create_dir_all(&repo).unwrap();
    fs::create_dir_all(&wt_out).unwrap();

    // Init repo
    git(&repo, &["init", "-b", "main"]);
    git(&repo, &["config", "user.email", "test@example.com"]);
    git(&repo, &["config", "user.name", "Test User"]);
    git(&repo, &["config", "commit.gpgsign", "false"]);

    // Seed files
    fs::write(repo.join(".env"), "SECRET=1\n").unwrap();
    fs::write(repo.join(".gitignore"), ".env\n").unwrap();
    fs::write(repo.join("README.md"), "# test repo\n").unwrap();

    // .worktree.toml — dir points INSIDE the tempdir (absolute), never ~/worktrees.
    let dir_template = format!("{}/{{name}}", wt_out.display());
    let copy_list = copy
        .iter()
        .map(|s| format!("\"{}\"", s))
        .collect::<Vec<_>>()
        .join(", ");
    let setup_list = setup
        .iter()
        .map(|s| format!("\"{}\"", s))
        .collect::<Vec<_>>()
        .join(", ");
    let toml = format!(
        "copy = [{copy_list}]\nsetup = [{setup_list}]\n\n[worktrees]\ndir = \"{dir_template}\"\nbranch_prefix = \"\"\n"
    );
    fs::write(repo.join(".worktree.toml"), toml).unwrap();

    // Commit everything (README + .gitignore; .env is gitignored).
    git(&repo, &["add", "README.md", ".gitignore", ".worktree.toml"]);
    git(&repo, &["commit", "-m", "init"]);

    Repo {
        _tmp: tmp,
        repo,
        wt_out,
    }
}

/// Set up a bare repo (no `.worktree.toml`) for the init test.
fn setup_repo_no_config() -> Repo {
    let tmp = tempfile::tempdir().expect("tempdir");
    let base = tmp.path();
    let repo = base.join("repo");
    let wt_out = base.join("out");
    fs::create_dir_all(&repo).unwrap();
    fs::create_dir_all(&wt_out).unwrap();

    git(&repo, &["init", "-b", "main"]);
    git(&repo, &["config", "user.email", "test@example.com"]);
    git(&repo, &["config", "user.name", "Test User"]);
    git(&repo, &["config", "commit.gpgsign", "false"]);

    fs::write(repo.join("README.md"), "# test repo\n").unwrap();
    git(&repo, &["add", "README.md"]);
    git(&repo, &["commit", "-m", "init"]);

    Repo {
        _tmp: tmp,
        repo,
        wt_out,
    }
}

/// True if branch `name` exists in `repo`.
fn branch_exists(repo: &Path, name: &str) -> bool {
    let out = StdCommand::new("git")
        .args(["branch", "--list", name])
        .current_dir(repo)
        .output()
        .unwrap();
    !String::from_utf8_lossy(&out.stdout).trim().is_empty()
}

#[test]
fn init_creates_and_force_overwrites() {
    let r = setup_repo_no_config();
    let cfg = r.repo.join(".worktree.toml");
    assert!(!cfg.exists());

    // First init succeeds and creates the file.
    wt(&r.repo).arg("init").assert().success();
    assert!(cfg.exists());

    // Second init fails without --force.
    wt(&r.repo).arg("init").assert().failure();

    // Second init succeeds with --force.
    wt(&r.repo).args(["init", "--force"]).assert().success();
}

#[test]
fn new_creates_worktree_copies_and_runs_setup() {
    let r = setup_repo_with(&[".env"], &["sh -c 'echo ran > setup-marker'"]);

    wt(&r.repo).args(["new", "fix-auth"]).assert().success();

    let wt_dir = r.wt_out.join("fix-auth");
    assert!(wt_dir.is_dir(), "worktree dir should exist");
    assert!(branch_exists(&r.repo, "fix-auth"), "branch should exist");
    assert!(wt_dir.join(".env").exists(), ".env should be copied");
    assert!(
        wt_dir.join("setup-marker").exists(),
        "setup command should have run"
    );
}

#[test]
fn new_no_setup_no_copy() {
    let r = setup_repo_with(&[".env"], &["sh -c 'echo ran > setup-marker'"]);

    wt(&r.repo)
        .args(["new", "feat", "--no-setup", "--no-copy"])
        .assert()
        .success();

    let wt_dir = r.wt_out.join("feat");
    assert!(wt_dir.is_dir(), "worktree dir should exist");
    assert!(!wt_dir.join(".env").exists(), ".env should NOT be copied");
    assert!(
        !wt_dir.join("setup-marker").exists(),
        "setup should NOT have run"
    );
}

#[test]
fn path_prints_and_missing_fails() {
    let r = setup_repo_with(&[], &[]);
    wt(&r.repo).args(["new", "fix-auth"]).assert().success();

    let wt_dir = r.wt_out.join("fix-auth");
    // git may canonicalize symlinks (e.g. /var -> /private/var on macOS);
    // compare by trailing component to stay robust.
    wt(&r.repo)
        .args(["path", "fix-auth"])
        .assert()
        .success()
        .stdout(predicate::str::ends_with(format!(
            "{}\n",
            wt_dir.file_name().unwrap().to_string_lossy()
        )));

    wt(&r.repo).args(["path", "nonexistent"]).assert().failure();
}

#[test]
fn new_print_path_prints_only_path() {
    let r = setup_repo_with(&[], &[]);

    let assert = wt(&r.repo)
        .args(["new", "x", "--print-path"])
        .assert()
        .success();
    let out = assert.get_output();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let trimmed = stdout.trim_end();

    // stdout must be exactly one line: the worktree path.
    assert_eq!(trimmed.lines().count(), 1, "stdout: {:?}", stdout);
    let wt_dir = r.wt_out.join("x");
    assert!(
        trimmed.ends_with(&wt_dir.file_name().unwrap().to_string_lossy().to_string()),
        "stdout {:?} should end with the worktree name",
        trimmed
    );
}

#[test]
fn ls_lists_worktree() {
    let r = setup_repo_with(&[], &[]);
    wt(&r.repo).args(["new", "fix-auth"]).assert().success();

    wt(&r.repo)
        .arg("ls")
        .assert()
        .success()
        .stdout(predicate::str::contains("fix-auth"));
}

#[test]
fn rm_removes_worktree_and_branch() {
    let r = setup_repo_with(&[], &[]);
    wt(&r.repo).args(["new", "fix-auth"]).assert().success();

    let wt_dir = r.wt_out.join("fix-auth");
    assert!(wt_dir.is_dir());
    assert!(branch_exists(&r.repo, "fix-auth"));

    wt(&r.repo)
        .args(["rm", "fix-auth", "--delete-branch"])
        .assert()
        .success();

    assert!(!wt_dir.exists(), "worktree dir should be removed");
    assert!(!branch_exists(&r.repo, "fix-auth"), "branch should be deleted");
}

#[test]
fn new_with_missing_copy_file_still_succeeds() {
    let r = setup_repo_with(&["missing.txt"], &[]);

    wt(&r.repo)
        .args(["new", "fix-auth"])
        .assert()
        .success();

    assert!(r.wt_out.join("fix-auth").is_dir());
}

#[test]
fn shellinit_zsh_prints_function() {
    // shellinit doesn't need a repo, but run somewhere harmless.
    let tmp = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("wt").unwrap();
    cmd.current_dir(tmp.path());
    cmd.args(["shellinit", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("builtin cd"))
        .stdout(predicate::str::contains("--print-path"));
}
