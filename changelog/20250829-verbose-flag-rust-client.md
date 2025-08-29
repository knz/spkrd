# Verbose Flag Implementation for Rust Client

## Task Specification
Modify the Rust client to only print informational messages when the `-v` flag is specified. Currently, the client likely prints various status and informational messages by default, and we need to gate these behind a verbose flag.

## Requirements
- Add `-v` or `--verbose` command line flag support
- Suppress informational messages by default
- Show informational messages only when verbose flag is active
- Preserve error messages regardless of verbose setting

## High-Level Decisions
- Added `-v`/`--verbose` flag using clap's derive macro for consistent CLI pattern
- Preserved all error messages (using eprintln!) to always display regardless of verbose setting
- Gated informational messages behind verbose flag for clean default output

## Files Modified
- `examples/client.rs`: Added verbose flag, conditional printing for informational messages, updated header comment

## Implementation Details
- Added `verbose: bool` field to `Args` struct with clap annotations
- Wrapped "Playing melody" and "Server" status messages in `if args.verbose` blocks
- Wrapped success confirmation message in verbose conditional
- Maintained all error handling and exit codes unchanged

## Current Status
- Implementation completed
- Client now runs silently by default, shows informational output with -v flag
- Error messages still display regardless of verbose setting