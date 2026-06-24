/// `wt shellinit [shell]` — print a shell integration function.
use anyhow::Result;

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Shell {
    Zsh,
    Bash,
    Fish,
}

impl Default for Shell {
    fn default() -> Self {
        Shell::Zsh
    }
}

const ZSH_BASH_FUNC: &str = r#"wt() {
  case "$1" in
    cd) builtin cd "$(command wt path "${2:-}")" || return ;;
    new) local p; p="$(command wt new "${@:2}" --print-path)" && builtin cd "$p" ;;
    *) command wt "$@" ;;
  esac
}"#;

const FISH_FUNC: &str = r#"function wt
  switch $argv[1]
    case cd
      builtin cd (command wt path $argv[2])
    case new
      set -l p (command wt new $argv[2..] --print-path)
      and builtin cd $p
    case '*'
      command wt $argv
  end
end"#;

/// Run the `shellinit` command.
pub fn run(shell: &Shell) -> Result<()> {
    match shell {
        Shell::Zsh | Shell::Bash => println!("{}", ZSH_BASH_FUNC),
        Shell::Fish => println!("{}", FISH_FUNC),
    }
    Ok(())
}
