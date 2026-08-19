# SPKRD - Speaker Network Server

A network server that exposes a melody-playback endpoint over HTTP. On
FreeBSD it can drive the kernel `/dev/speaker` device directly; on any
other host (Linux, macOS, Windows) it can synthesise the same melodies
through the system audio output via [CPAL].

[CPAL]: https://github.com/RustAudio/cpal

## Overview

SPKRD accepts FreeBSD-style melody strings over HTTP and plays them
back through one of two backends:

- **`freebsd-speaker`** — writes the melody to the kernel
  `/dev/speaker` character device. The kernel driver does the
  synthesis on the PC speaker hardware.
- **`cpal`** — parses the melody in user space (a faithful Rust port
  of the FreeBSD `spkr.c` interpreter) and renders it to audio via
  CPAL using a configurable waveform (square / band-limited square /
  sine / triangle / sawtooth).

In `--output=auto` (the default) the server probes the configured
device path and uses `freebsd-speaker` if it exists, falling back to
`cpal` otherwise. Both backends share the same HTTP surface, retry
logic, validation, and one-melody-at-a-time semantics.

## Features

- **HTTP API** - Simple PUT endpoint for melody playback
- **Two backends** - FreeBSD `/dev/speaker` or cross-platform CPAL audio output
- **Configurable Listen Addresses** - Bind any mix of IPv4 and IPv6 addresses and ports
- **Device Retry Logic** - Automatically retries when busy (1s intervals, configurable timeout)
- **Input Validation** - Configurable melody length limit and UTF-8 validation
- **Configurable Device Path** - Use custom device paths for testing or alternative devices
- **Daemon Support** - Run as background daemon with PID file management
- **Flexible Logging** - Syslog for daemon mode, stderr for foreground, with debug logging support
- **Request Logging** - Timestamps, client IPs, and printable melody content (debug mode only)
- **Example Clients** - Ready-to-use clients in Rust and Go

## Installation

```bash
git clone <repository-url>
cd spkrd
make && make install
```

For prerequisites, optional audio-backend features, and service setup on
FreeBSD (rc.d) and Linux (systemd), see **[INSTALL.md](INSTALL.md)**.

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

## Usage

```bash
# Start the server (listens on 0.0.0.0:1111 and [::]:1111)
spkrd

# Play a melody
curl -X PUT http://localhost:1111/play -d "cdefgab"
```

For the full command line reference, listen-address syntax, waveforms,
logging, and troubleshooting, see **[USAGE.md](USAGE.md)**. For the HTTP
API, see **[API.md](API.md)**.

## Development

For the project layout, test suite, build configurations, CI, and
contribution workflow, see **[DEVELOPMENT.md](DEVELOPMENT.md)**.

## How It Works

1. **HTTP Request** - Client sends PUT request to `/play` with melody data
2. **Validation** - Server validates melody length (against
   `--max-melody-length`, default 1000 bytes) and UTF-8 encoding
3. **Device Access** - Server attempts to open the speaker device
4. **Retry Logic** - If device is busy (EBUSY), retry every 1 second until timeout
5. **Playback** - Write melody to device and close
6. **Response** - Return appropriate HTTP status code

## License

This project is licensed under the BSD 2-Clause License. See the [LICENSE](LICENSE) file for details.

Copyright (c) 2025-2026, Raphael Poss

## Contributing

See [DEVELOPMENT.md](DEVELOPMENT.md#contributing).

## See Also

- [Installation instructions](INSTALL.md)
- [Usage, logging and troubleshooting](USAGE.md)
- [Development notes](DEVELOPMENT.md)
- [API Documentation](API.md)
- [Client examples](examples/README.md)
- [FreeBSD speaker(4) Manual](https://man.freebsd.org/cgi/man.cgi?query=speaker&apropos=0&sektion=0&manpath=FreeBSD+14.3-RELEASE+and+Ports&arch=default&format=html)
- [Project changelogs](changelog/)
