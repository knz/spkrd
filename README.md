# SPKRD - FreeBSD Speaker Network Server

A network server that provides HTTP access to FreeBSD's `/dev/speaker` device for remote melody playback.

## Overview

SPKRD exposes FreeBSD's built-in speaker device over HTTP, allowing you to play melodies remotely from any system that can make HTTP requests. The server handles device concurrency automatically with configurable retry logic.

## Features

- **HTTP API** - Simple PUT endpoint for melody playback
- **Device Retry Logic** - Automatically retries when device is busy (1s intervals, configurable timeout)
- **Input Validation** - Melody length limits and UTF-8 validation
- **Configurable Device Path** - Use custom device paths for testing or alternative devices
- **Daemon Support** - Run as background daemon with PID file management
- **Flexible Logging** - Syslog for daemon mode, stderr for foreground, with debug logging support
- **Request Logging** - Timestamps, client IPs, and printable melody content (debug mode only)
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

### System-Wide Installation

For production deployment as a system service on FreeBSD:

```bash
# Install to system directories (default: /usr/local)
make install

# Or install to custom location
make install DSTDIR=/usr/local

# Or install with custom program name
make install PROGRAM=my-spkrd
```

This installs:
- Binary to `/usr/local/bin/spkrd`
- FreeBSD rc.d script to `/usr/local/etc/rc.d/spkrd`

### Service Configuration

Add the following to `/etc/rc.conf` to enable the service:

```bash
# Enable the service
spkrd_enable="YES"

# Configure server options via flags
spkrd_flags="--port 8080 --device /dev/speaker --retry-timeout 30"
```

**Available configuration flags:**
- `--port <port>` - Server port (default: 8080)
- `--device <path>` - Speaker device path (default: /dev/speaker)  
- `--retry-timeout <secs>` - Device retry timeout (default: 30)
- `--daemon` - Run as background daemon (automatically added by rc.d)
- `--pidfile <path>` - PID file path (default: /var/run/spkrd.pid)
- `--debug/-d` - Enable debug logging including client request details

**Example configurations:**

```bash
# Custom port
spkrd_flags="--port 3000"

# Different device and port
spkrd_flags="--device /tmp/test-speaker --port 9000"

# Extended timeout
spkrd_flags="--retry-timeout 60 --port 8080"

# Enable debug logging (shows client requests in logs)
spkrd_flags="--debug --port 8080"

# Custom PID file location for non-root execution
spkrd_flags="--pidfile /tmp/spkrd.pid --port 8080"
```

### Service Management

```bash
# Start the service
service spkrd start

# Stop the service
service spkrd stop

# Restart the service
service spkrd restart

# Check service status
service spkrd status
```

### Logging

SPKRD supports flexible logging with different outputs depending on execution mode:

#### Daemon Mode (--daemon)
- Uses **syslog** with facility `daemon`
- Logs go to system log (typically `/var/log/daemon.log` or `/var/log/messages`)
- View logs: `tail -f /var/log/daemon.log | grep spkrd`

#### Foreground Mode (default)
- Uses **stderr** with timestamps
- Logs appear directly in terminal
- Suitable for development and manual testing

#### Log Levels
- **Default**: Startup messages (with all configuration) and errors only
- **Debug** (`--debug/-d`): Adds client request logging including:
  - Client IP address
  - Printable characters from melody data
  - Request status and retry count
  - Completion status

#### Examples

```bash
# View daemon logs on FreeBSD
tail -f /var/log/daemon.log | grep spkrd

# Run with debug logging in foreground
./spkrd --debug --port 8080

# Service with debug logging (via rc.conf)
spkrd_flags="--debug"
service spkrd restart
```

**Sample log output:**
```
# Startup (always logged)
Jan 29 10:30:15 hostname spkrd[1234]: Starting spkrd server: port=8080, retry_timeout=30s, device=/dev/speaker, daemon=true, pidfile=/var/run/spkrd.pid, debug=false

# Error (always logged)
Jan 29 10:30:16 hostname spkrd[1234]: Device error for request from 192.168.1.100: Permission denied

# Debug request logging (--debug only)
Jan 29 10:30:17 hostname spkrd[1234]: Request from 192.168.1.100: melody=t120l4cdefgab
Jan 29 10:30:17 hostname spkrd[1234]: Request from 192.168.1.100 completed successfully after 0 retries
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

# Run as daemon
./target/release/spkrd --daemon

# Run with debug logging
./target/release/spkrd --debug
```

### Command Line Options

- `--port` - Server port (default: 8080)
- `--retry-timeout` - Device retry timeout in seconds (default: 30)
- `--device` - Path to speaker device (default: /dev/speaker)
- `--daemon` - Run as background daemon
- `--pidfile` - Path to PID file (default: /var/run/spkrd.pid)
- `--debug/-d` - Enable debug logging including client request details

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

The `examples/` directory contains ready-to-use client implementations in Rust and Go.

**Quick Examples:**
```bash
# Rust client with config file
cd examples
echo "http://server:8080" > ~/.spkrc
./target/release/client "cdefgab"

# Go client
go run client.go http://server:8080 "cdefgab"
```

For complete client documentation, build instructions, and usage examples, see **[examples/README.md](examples/README.md)**.

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
