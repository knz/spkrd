# SPKRD - FreeBSD Speaker Network Server

A network server that provides HTTP access to FreeBSD's `/dev/speaker` device for remote melody playback.

## Overview

SPKRD exposes FreeBSD's built-in speaker device over HTTP, allowing you to play melodies remotely from any system that can make HTTP requests. The server handles device concurrency automatically with configurable retry logic.

## Features

- **HTTP API** - Simple PUT endpoint for melody playback
- **Device Retry Logic** - Automatically retries when device is busy (1s intervals, configurable timeout)
- **Input Validation** - Melody length limits and UTF-8 validation
- **Configurable Device Path** - Use custom device paths for testing or alternative devices
- **Request Logging** - Timestamps, client IPs, and printable melody content
- **Example Clients** - Ready-to-use clients in Rust and Go

## FreeBSD Speaker Device

The FreeBSD speaker device (`/dev/speaker`) accepts melody strings in a specific format. For complete documentation of the melody syntax, see the FreeBSD manual:

**[FreeBSD speaker(4) Manual Page](https://man.freebsd.org/cgi/man.cgi?query=speaker&apropos=0&sektion=0&manpath=FreeBSD+14.3-RELEASE+and+Ports&arch=default&format=html)**

### Quick Melody Syntax Reference

- **Notes:** `a`, `b`, `c`, `d`, `e`, `f`, `g` (with optional `#` or `+` for sharp)
- **Octaves:** `o1` to `o7` (default o4)
- **Length:** `l1`, `l2`, `l4`, `l8`, `l16`, `l32` (whole, half, quarter, etc.)
- **Tempo:** `t60` to `t255` (beats per minute)
- **Pause:** `p` followed by length
- **Repeat:** `.` after note extends by half

Example: `"t120l4 c d e f g a b o5c"`

## Installation

### Prerequisites

- Rust 1.70+ (for server)
- Go 1.19+ (for Go client example)
- FreeBSD system with `/dev/speaker` device

### Building

```bash
# Clone and build the server
git clone <repository-url>
cd spkrd
cargo build --release

# Build example clients
cd examples
cargo build --release  # Rust client
go build client.go      # Go client
```

## Usage

### Starting the Server

```bash
# Basic usage (default port 8080, device /dev/speaker)
./target/release/spkrd

# Custom configuration
./target/release/spkrd --port 3000 --retry-timeout 60 --device /dev/speaker

# For testing with a regular file
./target/release/spkrd --device /tmp/test-speaker
```

### Command Line Options

- `--port` - Server port (default: 8080)
- `--retry-timeout` - Device retry timeout in seconds (default: 30)
- `--device` - Path to speaker device (default: /dev/speaker)

### API Usage

#### Play a Melody

```bash
curl -X PUT http://localhost:8080/play -d "cdefgab"
```

#### Response Codes

- **200** - Melody played successfully (empty body)
- **400** - Invalid melody (error message in body)
- **503** - Device busy/timeout (error message in body)
- **500** - Server error (error message in body)

### Example Clients

#### Rust Client

```bash
cd examples
cargo run --bin client http://server:8080 "t120l8cdegreg"
```

#### Go Client

```bash
cd examples
go run client.go http://server:8080 "cdefgab"
```

#### Python Example

```python
import requests

response = requests.put('http://server:8080/play', data='cdefgab')
if response.status_code == 200:
    print("Melody played successfully")
else:
    print(f"Error: {response.text}")
```

## Development

### Running Tests

```bash
# Run all tests (uses temporary files as mock devices)
cargo test

# Run with verbose output
cargo test -- --nocapture
```

### Project Structure

```
spkrd/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Library interface
│   ├── server.rs        # HTTP server
│   ├── speaker.rs       # Device handling
│   └── error.rs         # Error types
├── tests/
│   └── integration_tests.rs  # Integration tests
├── examples/
│   ├── client.rs        # Rust client
│   ├── client.go        # Go client
│   └── Cargo.toml       # Client dependencies
├── API.md               # Detailed API documentation
└── README.md            # This file
```

## How It Works

1. **HTTP Request** - Client sends PUT request to `/play` with melody data
2. **Validation** - Server validates melody length (≤1000 chars) and UTF-8 encoding
3. **Device Access** - Server attempts to open the speaker device
4. **Retry Logic** - If device is busy (EBUSY), retry every 1 second until timeout
5. **Playback** - Write melody to device and close
6. **Response** - Return appropriate HTTP status code

## Troubleshooting

### Permission Denied

If you get permission errors accessing `/dev/speaker`:

```bash
# Check device permissions
ls -l /dev/speaker

# Add user to appropriate group (typically 'wheel' or 'operator')
sudo pw groupmod wheel -m username

# Or run with sudo (not recommended for production)
sudo ./target/release/spkrd
```

### Device Busy

The server automatically retries when the device is busy. If you consistently get timeout errors:

- Increase `--retry-timeout` value
- Check if another process is using the speaker device
- Verify the device path is correct

### Testing Without Hardware

Use a regular file as a mock device for testing:

```bash
# Start server with file device
./target/release/spkrd --device /tmp/test-speaker

# Send melody
curl -X PUT http://localhost:8080/play -d "cdefgab"

# Check result
cat /tmp/test-speaker
```

## License

This project is licensed under the BSD 2-Clause License. See the [LICENSE](LICENSE) file for details.

Copyright (c) 2025, Raphael Poss

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass with `cargo test`
5. Submit a pull request

## See Also

- [FreeBSD speaker(4) Manual](https://man.freebsd.org/cgi/man.cgi?query=speaker&apropos=0&sektion=0&manpath=FreeBSD+14.3-RELEASE+and+Ports&arch=default&format=html)
- [API Documentation](API.md)
- [Project Changelog](changelog/20250829-freebsd-speaker-server.md)
