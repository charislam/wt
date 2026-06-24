/// Configuration loading and merging.
/// Per-repo `.worktree.toml` overrides global `~/.config/wt/config.toml`.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Worktree placement configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorktreeCfg {
    /// Directory template; supports `{repo}` and `{name}`. May start with `~`.
    pub dir: Option<String>,
    /// Optional prefix prepended to branch names.
    pub branch_prefix: Option<String>,
}

/// Top-level configuration struct (matches `.worktree.toml`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Files to copy from the main worktree into a new worktree.
    #[serde(default)]
    pub copy: Vec<String>,
    /// Shell commands to run inside a new worktree after creation.
    #[serde(default)]
    pub setup: Vec<String>,
    /// Worktree placement settings.
    #[serde(default)]
    pub worktrees: WorktreeCfg,
}

impl Config {
    /// Merge `other` on top of `self` (per-repo overrides global).
    pub fn merge_with(mut self, other: Config) -> Config {
        if !other.copy.is_empty() {
            self.copy = other.copy;
        }
        if !other.setup.is_empty() {
            self.setup = other.setup;
        }
        if other.worktrees.dir.is_some() {
            self.worktrees.dir = other.worktrees.dir;
        }
        if other.worktrees.branch_prefix.is_some() {
            self.worktrees.branch_prefix = other.worktrees.branch_prefix;
        }
        self
    }
}

fn load_toml(path: &Path) -> Result<Config> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("reading config {}", path.display()))?;
    toml::from_str(&contents).with_context(|| format!("parsing config {}", path.display()))
}

/// Load and merge global + per-repo config.
/// `repo_root` is the top-level of the main worktree.
pub fn load(repo_root: &Path) -> Result<Config> {
    // Global config
    let global_cfg = dirs::config_dir()
        .map(|d| d.join("wt").join("config.toml"))
        .and_then(|p| if p.exists() { Some(p) } else { None });

    let base = if let Some(gp) = global_cfg {
        load_toml(&gp).unwrap_or_default()
    } else {
        Config::default()
    };

    // Per-repo config
    let local_path = repo_root.join(".worktree.toml");
    let local = if local_path.exists() {
        load_toml(&local_path).unwrap_or_default()
    } else {
        Config::default()
    };

    Ok(base.merge_with(local))
}

/// Resolve the worktree target directory from config.
/// `repo_root` used to derive `{repo}` (last component of the path).
pub fn resolve_dir(cfg: &Config, repo_root: &Path, name: &str) -> Result<PathBuf> {
    let template = cfg.worktrees.dir.clone().unwrap_or_else(|| {
        "~/worktrees/{repo}/{name}".to_string()
    });

    let repo_name = repo_root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "repo".to_string());

    let expanded = template
        .replace("{repo}", &repo_name)
        .replace("{name}", name);

    let expanded =
        shellexpand::tilde(&expanded).to_string();

    Ok(PathBuf::from(expanded))
}
