# FreeBSD Speaker Network Server - 20250829

## Task Specification
Create a network server for FreeBSD's `/dev/speaker` device that allows remote melody playback via HTTP API.

**Requirements:**
- Rust-based HTTP server with PUT handler
- Mutex-protected device access (single client at a time)
- HTTP-based protocol using PUT requests
- API documentation
- Example clients in Go and Rust

## High-Level Decisions
- Endpoint: PUT /play with melody in request body
- Melody validation: max 1000 characters
- Error handling: Driver enforces single client; retry on EBUSY with 1s intervals
- Retry mechanism: Configurable timeout (default 30s) via command line
- Configuration: Command-line arguments for port and retry timeout
- Responses: HTTP status codes only (success), plain text for errors
- Logging: Timestamp, client IP, printable characters from melody

## Requirements Changes
- **Device path configuration**: Added `--device` CLI parameter to specify custom device path (default: /dev/speaker)

## Files Modified
- **changelog/20250829-freebsd-speaker-server.md** (created) - Project tracking
- **Cargo.toml** (created) - Server dependencies and metadata
- **src/main.rs** (created) - CLI entry point with argument parsing  
- **src/error.rs** (created) - Custom error types for device operations
- **src/speaker.rs** (created) - Core speaker device handling with retry logic
- **src/server.rs** (created) - HTTP server using Axum framework
- **API.md** (created) - Complete API documentation with examples
- **examples/client.rs** (created) - Rust client example with error handling
- **examples/client.go** (created) - Go client example with status codes
- **examples/Cargo.toml** (created) - Dependencies for Rust client
- **src/lib.rs** (created) - Library interface exposing modules for testing
- **tests/integration_tests.rs** (created) - Integration tests using temp files as mock devices
- **README.md** (created) - Comprehensive project documentation with FreeBSD speaker manual reference
- **LICENSE** (created) - BSD 2-Clause license for open source distribution

## Rationales and Alternatives
- **No mutex needed**: FreeBSD driver handles device locking, simplifies architecture
- **Axum framework**: Modern async HTTP server with good ergonomics
- **Direct file I/O**: Simple open/write/close pattern for device access
- **1s retry interval**: Balance between responsiveness and resource usage
- **Separate error types**: Clear error handling and HTTP status mapping

## Obstacles and Solutions
- Device concurrency handled by driver (EBUSY detection and retry)
- UTF-8 validation for request bodies to prevent malformed data

## Current Status
- ✓ Full implementation complete
- ✓ Rust server with retry logic and configurable device path
- ✓ HTTP API with proper error handling
- ✓ API documentation with examples
- ✓ Rust and Go client examples
- ✓ CLI accepts custom device path via --device parameter
- ✓ Integration tests with temporary files as mock devices
- ✓ All tests passing (4/4)
- Ready for testing on FreeBSD system