# spkcmd-zsh.sh - Automatic audio feedback for zsh commands
#
# This configuration provides automatic audio feedback for command line
# operations using spkcmd. Commands are automatically wrapped with spkcmd
# to provide exit status audio feedback using zsh's native preexec function.
#
# Usage:
#   source /path/to/spkcmd-zsh.sh
#
# Control functions:
#   spkcmd_on  - Enable automatic audio feedback (default)
#   spkcmd_off - Disable automatic audio feedback

# Global state for spkcmd integration
_SPKCMD_ENABLED=1
# Track dynamically wrapped commands
typeset -A _SPKCMD_WRAPPED_COMMANDS

# Commands to exclude from spkcmd wrapping
_SPKCMD_EXCLUDE_BUILTINS=(
    "cd" "pwd" "echo" "printf" "true" "false" "test" "[" "eval"
    "exec" "exit" "return" "break" "continue" "shift" "unset"
    "export" "declare" "local" "readonly" "typeset" "alias" "unalias"
    "hash" "type" "which" "command" "builtin" "enable" "help"
    "history" "fc" "jobs" "bg" "fg" "disown" "kill" "wait"
    "read" "vared" "select" "caller" "source" "."
)

_SPKCMD_EXCLUDE_FAST=(
    "ls" "ll" "la" "cat" "head" "tail" "wc" "grep" "awk" "sed"
    "cut" "sort" "uniq" "tr" "find" "locate" "which" "whereis"
    "file" "stat" "du" "df" "free" "ps" "top" "uptime" "date"
    "whoami" "id" "groups" "uname" "hostname" "pwd"
)

_SPKCMD_EXCLUDE_INTERACTIVE=(
    "vim" "vi" "nano" "emacs" "ed" "pico" "joe" "micro"
    "less" "more" "most" "bat" "man" "info" "pager"
    "ssh" "scp" "sftp" "rsync" "ftp" "telnet" "nc" "netcat"
    "mysql" "psql" "sqlite3" "redis-cli" "mongo"
    "gdb" "lldb" "strace" "ltrace" "valgrind"
    "tmux" "screen" "byobu" "zellij"
    "htop" "btop" "iotop" "nethogs" "iftop"
    "watch" "tail" "journalctl" "dmesg"
)

# Check if command should be excluded from spkcmd wrapping
_spkcmd_should_exclude() {
    local cmd="$1"
    
    # Skip if spkcmd is disabled
    [[ $_SPKCMD_ENABLED -eq 0 ]] && return 0
    
    # Skip empty commands
    [[ -z "$cmd" ]] && return 0
    
    # Skip background jobs (commands ending with &)
    [[ "$cmd" =~ '&[[:space:]]*$' ]] && return 0
    
    # Skip if already using spkcmd
    [[ "$cmd" =~ '^[[:space:]]*spkcmd[[:space:]]' ]] && return 0
    
    # Extract first word (command name)
    local first_word=${cmd%% *}
    first_word=${first_word##*( )}
    
    # Check against exclusion lists
    if (( ${_SPKCMD_EXCLUDE_BUILTINS[(Ie)$first_word]} )); then
        return 0
    fi
    if (( ${_SPKCMD_EXCLUDE_FAST[(Ie)$first_word]} )); then
        return 0
    fi
    if (( ${_SPKCMD_EXCLUDE_INTERACTIVE[(Ie)$first_word]} )); then
        return 0
    fi
    
    return 1
}

# Check if command needs a dynamic wrapper
_spkcmd_needs_wrapper() {
    local cmd="$1"
    
    # Skip if spkcmd is disabled
    [[ $_SPKCMD_ENABLED -eq 0 ]] && return 1
    
    # Skip if already wrapped (function exists)
    (( $+functions[$cmd] )) && return 1
    
    # Skip if it's an alias
    (( $+aliases[$cmd] )) && return 1
    
    # Skip if it's a builtin
    [[ $(whence -w "$cmd") == *": builtin" ]] && return 1
    
    # Only wrap if it's an external command that exists
    [[ $(whence -w "$cmd") == *": command" ]] && command -v "$cmd" >/dev/null 2>&1
}

# Store original preexec if it exists and isn't already our spkcmd preexec
if (( $+functions[preexec] )) && [[ -z "$_SPKCMD_PREEXEC_LOADED" ]]; then
    functions[_spkcmd_original_preexec]=$functions[preexec]
fi

# preexec function - called before each command execution
preexec() {
    # Prevent recursion on reload
    [[ -n "$_SPKCMD_PREEXEC_ACTIVE" ]] && return
    _SPKCMD_PREEXEC_ACTIVE=1
    
    # Call original preexec if it existed
    if (( $+functions[_spkcmd_original_preexec] )); then
        _spkcmd_original_preexec "$@"
    fi
    
    local cmd="$1"
    
    # Extract first word (command name)
    local first_word=${cmd%% *}
    first_word=${first_word##*( )}
    
    # Check if we should exclude this command
    if _spkcmd_should_exclude "$cmd"; then
        unset _SPKCMD_PREEXEC_ACTIVE
        return
    fi
    
    # Check if this command needs a dynamic wrapper
    if _spkcmd_needs_wrapper "$first_word"; then
        # Create wrapper function dynamically for next execution
        eval "$first_word() { spkcmd =\$0 \"\$@\"; }"
        # Track the wrapped command  
        _SPKCMD_WRAPPED_COMMANDS[$first_word]=1
    fi
    
    # Clear the recursion guard
    unset _SPKCMD_PREEXEC_ACTIVE
}


# Create wrapper functions for common commands (pre-seeding for immediate availability)
_spkcmd_create_wrappers() {
    # Pre-seed a few very common commands for immediate availability
    # Additional commands will be wrapped dynamically on first use in interactive sessions
    local priority_commands=(
        "make" "git" "cargo"
    )
    
    local cmd
    for cmd in $priority_commands; do
        if _spkcmd_needs_wrapper "$cmd"; then
            eval "$cmd() { spkcmd =\$0 \"\$@\"; }"
            _SPKCMD_WRAPPED_COMMANDS[$cmd]=1
        fi
    done
}

# Remove wrapper functions
_spkcmd_remove_wrappers() {
    # Remove all tracked wrapped commands
    local cmd
    for cmd in ${(k)_SPKCMD_WRAPPED_COMMANDS}; do
        if (( $+functions[$cmd] )); then
            unfunction $cmd
        fi
    done
    # Clear the tracking
    _SPKCMD_WRAPPED_COMMANDS=()
}

# Control functions
spkcmd_on() {
    _SPKCMD_ENABLED=1
    _spkcmd_create_wrappers
    echo "spkcmd audio feedback enabled (dynamic wrapping active)"
}

spkcmd_off() {
    _SPKCMD_ENABLED=0
    _spkcmd_remove_wrappers
    echo "spkcmd audio feedback disabled"
}

# Initialize spkcmd integration
if [[ $_SPKCMD_ENABLED -eq 1 ]]; then
    _spkcmd_create_wrappers
    echo "spkcmd zsh integration loaded (use 'spkcmd_off' to disable)"
fi

# Mark preexec as loaded to prevent recursion on reload
_SPKCMD_PREEXEC_LOADED=1