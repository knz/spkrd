# RC Script Boot Issue Investigation

## Task Specification
User reported that the spkrd daemon was not starting on boot with the provided rc.d script. They manually added rcorder comments and want verification that the script is now correct.

## Initial Analysis
The rc.d script now includes rcorder comments:
- `PROVIDE: spkrd`
- `REQUIRE: DAEMON`
- `KEYWORD: shutdown`

## Files to Review
- `rc.d/spkrd`: FreeBSD service script

## Current Status
- Examining script for correctness
- Need to verify rcorder comments are appropriate
- Check for other potential boot startup issues

## Issues Found
1. **Variable loading order**: `load_rc_config` called after `command_args` definition, preventing `spkrd_flags` from being properly loaded from `/etc/rc.conf`
2. **rcorder comment placement**: User prefers rcorder comments after the explanatory header comment

## Implementation Plan

### Fix 1: Correct Variable Loading Order
Move `load_rc_config $name` to execute before `command_args` is set. Standard pattern:
```sh
. /etc/rc.subr
name=spkrd
rcvar=spkrd_enable
load_rc_config $name  # Load user config first
# Then set defaults and use variables
```

### Fix 2: Move rcorder Comments
Place `PROVIDE`/`REQUIRE`/`KEYWORD` comments after the explanatory header block. This is supported by FreeBSD - rcorder scans the entire file for these special comments, they don't need to be at the top.

## Changes Applied
- Moved rcorder comments (lines 3-5) to after explanatory header (after line 34)
- Moved `load_rc_config $name` to before `command` and `command_args` definitions
- Added `: ${spkrd_enable:="NO"}` default for cleaner rc.conf integration
