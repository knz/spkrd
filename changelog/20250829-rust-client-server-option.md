# Rust Client Server Configuration Enhancement

## Task Specification
Modify the Rust client to:
- Accept server name/URL via command line option (not positional parameter)
- Fall back to reading server URL from ~/.spkrc when not specified via command line
- Remove current positional parameter approach

## High-Level Decisions
- Used existing `clap` dependency with derive macros for robust CLI parsing
- Implemented precedence: command line option > config file > error
- Simple plain text config file format (just the URL)
- Maintained melody as required positional argument
- Added helpful error messages with examples

## Requirements Changes
- Config file path corrected to ~/.spkrc (with dot prefix)

## Files Modified
- `examples/client.rs`: Complete rewrite of argument parsing and server URL resolution
  - Replaced manual env::args() with clap Parser derive
  - Added --server/-s option with config file fallback
  - Added config file reading from ~/.spkrc
  - Improved error handling and user messages
- `examples/Cargo.toml`: Added clap dependency for CLI parsing
- `examples/README.md`: Enhanced with comprehensive client documentation
  - Added detailed Rust client usage with command line options and config file
  - Documented configuration priority and all available options
  - Added Go client usage examples
  - Included example melodies and syntax reference
- `README.md`: Simplified client section to reference examples/README.md
  - Replaced detailed client documentation with brief examples
  - Added clear reference to examples/README.md for full documentation
  - Maintained Python example in main README

## Rationales and Alternatives
- Chose clap derive over manual parsing for robustness and maintainability
- Simple text config file over structured format for ease of use
- Command line overrides config file following standard CLI conventions
- Clear error messages help users understand configuration options

## Obstacles and Solutions
- None encountered - implementation was straightforward

## Current Status
- ✅ Implementation completed successfully
- ✅ All functionality tested and working:
  - Command line --server option works
  - Config file ~/.spkrc fallback works
  - Command line overrides config file correctly
  - Error handling when no server configured
  - Help output is clear and helpful
- ✅ Build system integration:
  - Added clap dependency to examples/Cargo.toml
  - `make` command in examples/ directory builds successfully
  - Release binary works correctly with actual server