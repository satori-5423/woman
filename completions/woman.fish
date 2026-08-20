# Fish completions for woman — AI-translated man pages
#
# Install: copy to ~/.config/fish/completions/woman.fish
# or run:  cp completions/woman.fish ~/.config/fish/completions/

# Helper to detect whether we are inside the "model" subcommand
# (either `woman model ...` or `woman --model ...`).
function __woman_in_model_mode
    set -l tokens (commandline -opc)
    for t in $tokens
        if test "$t" = model -o "$t" = --model
            return 0
        end
    end
    return 1
end

# ---- Man page names (reuse fish's built-in completion) ----
complete -xc woman -a "(__fish_complete_man)"

# ---- Section numbers (offered when page name completion yields nothing) ----
complete -xc woman -n 'not __woman_in_model_mode; and not __fish_complete_man' -a 1  -d 'Program section'
complete -xc woman -n 'not __woman_in_model_mode; and not __fish_complete_man' -a 1p -d 'POSIX program section'
complete -xc woman -n 'not __woman_in_model_mode; and not __fish_complete_man' -a 2  -d 'Syscall section'
complete -xc woman -n 'not __woman_in_model_mode; and not __fish_complete_man' -a 3  -d 'Library section'
complete -xc woman -n 'not __woman_in_model_mode; and not __fish_complete_man' -a 3p -d 'POSIX library section'
complete -xc woman -n 'not __woman_in_model_mode; and not __fish_complete_man' -a 4  -d 'Device section'
complete -xc woman -n 'not __woman_in_model_mode; and not __fish_complete_man' -a 5  -d 'File format section'
complete -xc woman -n 'not __woman_in_model_mode; and not __fish_complete_man' -a 6  -d 'Games section'
complete -xc woman -n 'not __woman_in_model_mode; and not __fish_complete_man' -a 7  -d 'Misc section'
complete -xc woman -n 'not __woman_in_model_mode; and not __fish_complete_man' -a 8  -d 'Admin section'
complete -xc woman -n 'not __woman_in_model_mode; and not __fish_complete_man' -a 9  -d 'Kernel section'

# ---- Flags ----
complete -c woman -n 'not __woman_in_model_mode' -s w -l path -l where -d 'Print translated page path instead of displaying'

# ---- model subcommand ----
complete -c woman -n 'not __woman_in_model_mode' -a model  -d 'Manage API model configuration'
complete -c woman -n 'not __woman_in_model_mode' -l model -d 'Manage API model configuration'

# model sub-subcommands (only offered before one is picked)
complete -c woman -n '__woman_in_model_mode; and not __fish_seen_subcommand_from list show set-model set-key set-url set-thinking set-effort' -a list      -d 'List available models'
complete -c woman -n '__woman_in_model_mode; and not __fish_seen_subcommand_from list show set-model set-key set-url set-thinking set-effort' -a show      -d 'Show current configuration'
complete -c woman -n '__woman_in_model_mode; and not __fish_seen_subcommand_from list show set-model set-key set-url set-thinking set-effort' -a set-model -d 'Set the active model name'
complete -c woman -n '__woman_in_model_mode; and not __fish_seen_subcommand_from list show set-model set-key set-url set-thinking set-effort' -a set-key   -d 'Set the API key'
complete -c woman -n '__woman_in_model_mode; and not __fish_seen_subcommand_from list show set-model set-key set-url set-thinking set-effort' -a set-url   -d 'Set the API base URL'
complete -c woman -n '__woman_in_model_mode; and not __fish_seen_subcommand_from list show set-model set-key set-url set-thinking set-effort' -a set-thinking -d 'Enable/disable thinking mode (enabled|disabled)'
complete -c woman -n '__woman_in_model_mode; and not __fish_seen_subcommand_from list show set-model set-key set-url set-thinking set-effort' -a set-effort -d 'Set reasoning effort (low|high|max)'

# After the value-taking subcommands: suppress file completion (free-text input).
complete -c woman -n '__woman_in_model_mode; and __fish_seen_subcommand_from set-model set-key set-url set-thinking set-effort' -f
