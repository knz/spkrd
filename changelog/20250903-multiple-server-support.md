# Multiple Server Support - 2025-09-03

## Task Specification
Update the client to support multiple server addresses, both on the command line and in configuration files. In configuration files, multiple servers should be specified on different lines.

## High-Level Decisions
- Use Vec<String> for server collections throughout the codebase
- Implement concurrent requests using tokio::spawn and join_all
- CLI servers completely override config file servers (no merging)
- Maintain backwards compatibility with existing single-server setups
- Preserve existing URL normalization per server

## Requirements Changes
- Broadcast behavior: Send melody to all servers simultaneously
- Command line: Multiple `--server` flags supported
- CLI completely overrides config file servers when present
- Only update Rust client (not Go client)
- Error reporting: Show all failures but exit success if any server succeeds

## Files Modified
- `examples/client.rs`: Complete refactor for multiple server support
  - Updated Args struct to use Vec<String> with ArgAction::Append
  - Modified config reading to parse multiple lines
  - Added ServerResult struct for tracking per-server results
  - Implemented concurrent broadcast using tokio::spawn
  - Updated error handling and exit code logic
- `examples/Cargo.toml`: Added futures dependency for join_all

## Rationales and Alternatives
- Chose tokio::spawn over futures::join_all directly for better error isolation
- Used ArgAction::Append instead of multiple value parsing for cleaner CLI
- Maintained existing URL normalization per server for consistency
- Config file uses line-based format (simple to parse, supports comments)

## Obstacles and Solutions
- futures crate needed for join_all - added to Cargo.toml
- Server result tracking - created ServerResult struct for clean state management
- Verbose output order - messages can interleave due to concurrency (acceptable)

## Current Status
- ✅ All tasks completed successfully
- ✅ Code compiles and runs correctly
- ✅ Tested with multiple CLI servers and config file
- ✅ Verified broadcast behavior and error reporting
- ✅ Confirmed exit codes work as specified