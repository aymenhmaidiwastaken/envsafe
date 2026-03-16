use anyhow::{bail, Result};

pub fn execute(shell: &str) -> Result<()> {
    let script = match shell {
        "bash" => {
            r#"
_envsafe_hook() {
    if [ -f ".envsafe/config.json" ]; then
        eval "$(envsafe export)"
    fi
}

if [[ ! "$PROMPT_COMMAND" =~ _envsafe_hook ]]; then
    PROMPT_COMMAND="_envsafe_hook;$PROMPT_COMMAND"
fi
"#
        }
        "zsh" => {
            r#"
_envsafe_hook() {
    if [ -f ".envsafe/config.json" ]; then
        eval "$(envsafe export)"
    fi
}

autoload -Uz add-zsh-hook
add-zsh-hook chpwd _envsafe_hook

# Also run on shell startup
_envsafe_hook
"#
        }
        "fish" => {
            r#"
function _envsafe_hook --on-variable PWD --description "Auto-inject envsafe env vars"
    if status --is-interactive; and test -f .envsafe/config.json
        envsafe export | source
    end
end

# Also run on shell startup
_envsafe_hook
"#
        }
        _ => bail!("Unsupported shell '{}'. Supported: bash, zsh, fish", shell),
    };

    print!("{}", script.trim_start_matches('\n'));
    Ok(())
}
