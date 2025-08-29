# RC Script Pidfile Fix - 2025-08-29

## Task Specification
Fix the FreeBSD rc script for spkrd service that fails with "a value is required for '--pidfile <PIDFILE>' but none was supplied" when running `service spkrd start`.

## Current Status
- Issue identified: rc script variable ordering problem affecting pidfile argument
- Need to examine rc.d/spkrd script structure
- Need to understand how FreeBSD rc scripts should pass arguments to daemon processes

## Issue Analysis
- Found the problem: In rc.d/spkrd line 38, `command_args` references `${pidfile}` before `pidfile` is defined on line 40
- Shell variables must be defined before they're referenced
- This causes the pidfile argument to be empty when the command is executed

## Implementation
- **Files Modified**: `rc.d/spkrd`
- **Change**: Moved `pidfile="/var/run/${name}.pid"` from line 40 to line 38, before `command_args` definition
- **Rationale**: Shell variables must be defined before they're referenced; the original ordering caused empty pidfile argument

## Resolution
- Fixed variable ordering issue that caused "a value is required for '--pidfile <PIDFILE>'" error
- Maintained default pidfile location `/var/run/spkrd.pid`
- No functional changes to rc script behavior, just proper variable sequencing