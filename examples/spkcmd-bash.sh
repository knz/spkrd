# spkcmd-bash.sh - Automatic audio feedback for bash commands
#
# This configuration provides automatic audio feedback for command line
# operations using spkcmd. Commands are automatically wrapped with spkcmd
# to provide exit status audio feedback.
#
# Usage:
#   source /path/to/spkcmd-bash.sh
#
# Control functions:
#   spkcmd_on  - Enable automatic audio feedback (default)
#   spkcmd_off - Disable automatic audio feedback

# Global state for spkcmd integration
_SPKCMD_ENABLED=1
# Track dynamically wrapped commands
declare -A _SPKCMD_WRAPPED_COMMANDS

# Commands to exclude from spkcmd wrapping
_SPKCMD_EXCLUDE_BUILTINS=(
    "cd" "pwd" "echo" "printf" "true" "false" "test" "[" "eval"
    "exec" "exit" "return" "break" "continue" "shift" "unset"
    "export" "declare" "local" "readonly" "typeset" "alias" "unalias"
    "hash" "type" "which" "command" "builtin" "enable" "help"
    "history" "fc" "jobs" "bg" "fg" "disown" "kill" "wait"
    "read" "mapfile" "readarray" "caller" "source" "."
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
    [[ "$cmd" == *" &"* ]] && return 0
    
    # Skip if already using spkcmd
    [[ "$cmd" == "spkcmd "* ]] && return 0
    
    # Extract first word (command name)
    local first_word
    first_word=$(echo "$cmd" | awk '{print $1}')
    
    # Check against exclusion lists
    local exclude_cmd
    for exclude_cmd in "${_SPKCMD_EXCLUDE_BUILTINS[@]}" "${_SPKCMD_EXCLUDE_FAST[@]}" "${_SPKCMD_EXCLUDE_INTERACTIVE[@]}"; do
        [[ "$first_word" == "$exclude_cmd" ]] && return 0
    done
    
    return 1
}

# Check if command needs a dynamic wrapper
_spkcmd_needs_wrapper() {
    local cmd="$1"
    
    # Skip if spkcmd is disabled
    [[ $_SPKCMD_ENABLED -eq 0 ]] && return 1
    
    # Skip if already wrapped (function exists)
    [[ $(type -t "$cmd") == "function" ]] && return 1
    
    # Skip if it's an alias
    [[ $(type -t "$cmd") == "alias" ]] && return 1
    
    # Skip if it's a builtin
    [[ $(type -t "$cmd") == "builtin" ]] && return 1
    
    # Only wrap if it's an external command that exists
    [[ $(type -t "$cmd") == "file" ]] && command -v "$cmd" >/dev/null 2>&1
}

# preexec function for bash - called before each command execution
preexec() {
    local cmd="$1"
    
    # Check if we should exclude this command
    if _spkcmd_should_exclude "$cmd"; then
        return
    fi
    
    # Modify the command line to use spkcmd
    # This is a bit tricky in bash - we'll use a different approach
    # Store the original command for potential wrapping
    _SPKCMD_ORIGINAL_CMD="$cmd"
}

# Since bash doesn't have native preexec, we need to implement it
# This implementation uses DEBUG trap and PROMPT_COMMAND
_spkcmd_debug_trap() {
    # Only process if we're in an interactive shell
    [[ $- == *i* ]] || return
    
    # Get the current command from history
    local cmd
    cmd=$(HISTTIMEFORMAT= history 1 | sed 's/^[ ]*[0-9]*[ ]*//')
    
    # Skip if command hasn't changed
    [[ "$cmd" == "$_SPKCMD_LAST_CMD" ]] && return
    _SPKCMD_LAST_CMD="$cmd"
    
    # Check if we should exclude this command
    if _spkcmd_should_exclude "$cmd"; then
        return
    fi
    
    # For bash, we'll use a different approach - override common commands with functions
    # This is set up in the initialization below
}

# Create wrapper functions for common commands (pre-seeding for immediate availability)
_spkcmd_create_wrappers() {
    # Pre-seed a few very common commands for immediate availability
    # Additional commands will be wrapped dynamically on first use in interactive sessions
    local priority_commands=(
        "make" "git" "cargo"
    )
    
    local cmd
    for cmd in "${priority_commands[@]}"; do
        if _spkcmd_needs_wrapper "$cmd"; then
            eval "$cmd() { spkcmd command $cmd \"\$@\"; }"
            _SPKCMD_WRAPPED_COMMANDS[$cmd]=1
        fi
    done
}

# Remove wrapper functions
_spkcmd_remove_wrappers() {
    # Remove all tracked wrapped commands
    local cmd
    for cmd in "${!_SPKCMD_WRAPPED_COMMANDS[@]}"; do
        if [[ $(type -t "$cmd") == "function" ]]; then
            unset -f "$cmd"
        fi
    done
    # Clear the tracking
    _SPKCMD_WRAPPED_COMMANDS=()
}

# Dynamic wrapper creation using DEBUG trap (interactive shells only)
_spkcmd_debug_trap() {
    # Only process if we're in an interactive shell
    [[ $- == *i* ]] || return
    
    # Get the current command from history
    local cmd
    cmd=$(HISTTIMEFORMAT= history 1 | sed 's/^[ ]*[0-9]*[ ]*//')
    
    # Skip if command hasn't changed
    [[ "$cmd" == "$_SPKCMD_LAST_CMD" ]] && return
    _SPKCMD_LAST_CMD="$cmd"
    
    # Extract first word (command name)
    local first_word
    first_word=$(echo "$cmd" | awk '{print $1}')
    
    # Check if we should exclude this command
    if _spkcmd_should_exclude "$cmd"; then
        return
    fi
    
    # Check if this command needs a dynamic wrapper
    if _spkcmd_needs_wrapper "$first_word"; then
        # Create wrapper function dynamically for next execution
        eval "$first_word() { spkcmd command $first_word \"\$@\"; }"
        # Track the wrapped command
        _SPKCMD_WRAPPED_COMMANDS[$first_word]=1
    fi
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
    # Set up DEBUG trap for dynamic wrapping (works in interactive shells)
    trap '_spkcmd_debug_trap' DEBUG
    echo "spkcmd bash integration loaded (use 'spkcmd_off' to disable)"
fi