# Using SPKRD

How to run and operate the SPKRD server: command line options, the
HTTP API, logging, and troubleshooting. For installing it, see
[INSTALL.md](INSTALL.md); for what it is, see [README.md](README.md);
for the full HTTP API reference, see [API.md](API.md).


## Starting the Server

```bash
# Basic usage (listens on 0.0.0.0:1111 and [::]:1111, device /dev/speaker)
./target/release/spkrd

# Custom configuration
./target/release/spkrd --port 3000 --retry-timeout 60 --device /dev/speaker

# Listen on localhost only (quote it: [ ] are shell glob characters)
./target/release/spkrd --bind '127.0.0.1,[::1]'

# Listen on a specific interface address and a second port
./target/release/spkrd --bind 192.168.1.10,127.0.0.1:9000

# For testing with a regular file
./target/release/spkrd --device /tmp/test-speaker

# Run as daemon
./target/release/spkrd --daemon

# Run with debug logging
./target/release/spkrd --debug
```

## Command Line Options

- `--bind <spec>` - Comma-separated list of listen addresses (default:
  `0.0.0.0,[::]`). See [Listen addresses](#listen-addresses) below.
- `--port <port>` / `-p` - Default port for `--bind` entries that omit one
  (default: 1111)
- `--retry-timeout <secs>` / `-r` - Device retry timeout in seconds (default: 30)
- `--max-melody-length <bytes>` - Maximum melody body length in bytes; must be
  in `1..=1048576` (default: 1000)
- `--device <path>` / `-d` - Path to speaker device, used by the
  `freebsd-speaker` backend (default: /dev/speaker)
- `--output <mode>` - Output backend: `auto` (default), `freebsd-speaker`, or
  `cpal` (the `cpal` value is available only when built with the `cpal` feature)
- `--daemon` - Run as background daemon
- `--pidfile <path>` - Path to PID file (default: /var/run/spkrd.pid)
- `--debug` / `-D` - Enable debug logging including client request details

Note that the short option for debug logging is `-D`; `-d` is `--device`.

**CPAL-only flags** (present only when built with the `cpal` feature, and
otherwise hidden):

- `--waveform <wf>` - `pc-speaker` (default), `square-bandlimited` (sounds nice),
  `square`, `sine`, `triangle`, or `sawtooth`. See [Waveforms](#waveforms).
- `--volume <v>` - Output volume in `[0.0, 1.0]` (default: 0.25)
- `--sample-rate <hz>` - Override the device's default sample rate
- `--cpal-host <name>` - CPAL host backend; matching is case-insensitive. Valid values:
  `ALSA` (default on Linux), `PipeWire` (requires `--features pipewire`),
  `PulseAudio` (requires `--features pulseaudio`), `JACK` (requires `--features jack`),
  `CoreAudio` (macOS), `WASAPI` (Windows). When omitted, cpal picks the best available
  host automatically (PipeWire > PulseAudio > ALSA on Linux).
- `--cpal-device <name>` - Output device name; defaults to the host's default output

## Listen addresses

`--bind` takes a comma-separated list of addresses. Each entry is either:

- a bare IPv4 literal, optionally suffixed `:port` — `0.0.0.0`, `127.0.0.1:9000`
- a bracketed IPv6 literal, optionally suffixed `:port` — `[::]`, `[::1]:9000`

Brackets are only valid around an IPv6 address; an unbracketed IPv6
literal is rejected, because its own colons cannot be told apart from the
optional `:port` suffix. An entry with no `:port` uses `--port`. The
server binds and listens on every address in the list.

**IPv6 entries are bound v6-only** (`IPV6_V6ONLY`). `[::]` therefore
serves IPv6 clients only and does not also accept IPv4 — list `0.0.0.0`
alongside it to serve both, as the default `0.0.0.0,[::]` does. Without
this, the two wildcard entries of the default would overlap on hosts
where a `[::]` socket is dual-stack (Linux with
`net.ipv6.bindv6only=0`), and the second bind would fail with
`EADDRINUSE`. Setting the option explicitly makes the default spec mean
the same thing on FreeBSD and Linux alike.

Note that `[` and `]` are glob characters in most shells — zsh in
particular will fail with "no matches found" on an unquoted IPv6 entry.
Quote any `--bind` argument containing brackets:

```bash
spkrd --bind '127.0.0.1,[::1]'
```

Examples (quoting omitted below for readability):

```bash
# Default: all interfaces, both families, port 1111
--bind 0.0.0.0,[::]

# Localhost only, both families
--bind 127.0.0.1,[::1]

# IPv6 only
--bind [::]

# One interface on the default port, plus localhost on another port
--bind 192.168.1.10,127.0.0.1:9000
```

## Waveforms

The `pc-speaker` waveform is a faithful simulation of a modern
piezoelectric PC speaker: note frequencies are quantised to what the
Intel 8254 PIT can actually produce (`1,193,182 Hz / divisor`, integer
divisor), a square wave at that frequency is processed through a 3-stage
biquad chain (high-pass / midrange peak / low-pass) tuned to a small
piezo disc, and the output is soft-clipped via `tanh` to mimic driver
saturation. The square-wave phase is reset at every note (mirroring the
PIT counter reset the FreeBSD kernel performs in `timer_spkr_setfreq`),
so consecutive notes — even at the same pitch — get the mechanical
"plink" articulation a real piezo produces. Filter state is preserved
across notes and rests, so the speaker rings out naturally on note-off
rather than cutting silently.

The `square` waveform is the kernel-faithful raw output: phase is reset
at every note (matching the PIT counter reset) and no envelope is
applied, so consecutive notes have hard amplitude-step boundary clicks
that match what FreeBSD's unfiltered `/dev/speaker` output sounds like
through a modern DAC. If you want click-suppressed alternatives, the
remaining software waveforms (`square-bandlimited`, `sine`, `triangle`,
`sawtooth`) keep phase continuity across notes and apply a 5 ms
attack/release envelope to fade in/out each note.

## HTTP API

### Play a Melody

```bash
curl -X PUT http://localhost:1111/play -d "cdefgab"
```

### Response Codes

- **200** - Melody played successfully (empty body)
- **400** - Invalid melody (error message in body)
- **503** - Device busy/timeout (error message in body)
- **500** - Server error (error message in body)

For the complete HTTP API, see **[API.md](API.md)**.

### Python Example

```python
import requests

response = requests.put('http://server:1111/play', data='cdefgab')
if response.status_code == 200:
    print("Melody played successfully")
else:
    print(f"Error: {response.text}")
```

## Example Clients

The `examples/` directory contains ready-to-use client implementations in
Rust and Go, a `spkcmd` audio-feedback wrapper, shell integrations, and a
collection of bundled `.mml` tunes.

**Quick Examples:**
```bash
# Rust client with config file
cd examples
echo "http://server:1111" > ~/.spkrc
./target/release/client "cdefgab"

# Go client
go run client.go http://server:1111 "cdefgab"
```

For complete client documentation, build instructions, and usage examples, see **[examples/README.md](examples/README.md)**.

## Logging

SPKRD supports flexible logging with different outputs depending on execution mode:

### Daemon Mode (--daemon)
- Uses **syslog** with facility `daemon`
- Logs go to system log (typically `/var/log/daemon.log` or `/var/log/messages`)
- View logs: `tail -f /var/log/daemon.log | grep spkrd`
- Under a systemd user unit, logs go to journald instead:
  `journalctl --user -u spkrd -f`

### Foreground Mode (default)
- Uses **stderr** with timestamps
- Logs appear directly in terminal
- Suitable for development and manual testing

### Log Levels
- **Default**: Startup messages (with all configuration) and errors only
- **Debug** (`--debug`/`-D`): Adds client request logging including:
  - Client IP address
  - Printable characters from melody data
  - Request status and retry count
  - Completion status

### Examples

```bash
# View daemon logs on FreeBSD
tail -f /var/log/daemon.log | grep spkrd

# View logs under a systemd user unit on Linux
journalctl --user -u spkrd -f

# Run with debug logging in foreground
./spkrd --debug --port 1111
```

**Sample log output:**
```
# Startup (always logged)
Jan 29 10:30:15 hostname spkrd[1234]: Starting spkrd: bind=[0.0.0.0:1111, [::]:1111], retry_timeout=30s, max_melody_length=1000, output=Auto (resolved=FreebsdSpeaker), device=/dev/speaker, daemon=true, pidfile=/var/run/spkrd.pid, debug=false

# Per-listener bind confirmation (always logged)
Jan 29 10:30:15 hostname spkrd[1234]: Server listening on 0.0.0.0:1111
Jan 29 10:30:15 hostname spkrd[1234]: Server listening on [::]:1111

# Error (always logged)
Jan 29 10:30:16 hostname spkrd[1234]: Device error for request from 192.168.1.100: Permission denied

# Debug request logging (--debug only)
Jan 29 10:30:17 hostname spkrd[1234]: Request from 192.168.1.100: melody=t120l4cdefgab
Jan 29 10:30:17 hostname spkrd[1234]: Request from 192.168.1.100 completed successfully after 0 retries
```

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

### Address Already in Use

If a bind fails with `EADDRINUSE`, another process holds the port — or
two entries in your `--bind` spec overlap. Note that `0.0.0.0` and `[::]`
do *not* overlap, since IPv6 listeners are bound v6-only; see
[Listen addresses](#listen-addresses).

### Device Busy

The server automatically retries when the device is busy. If you consistently get timeout errors:

- Increase `--retry-timeout` value
- Check if another process is using the speaker device
- Verify the device path is correct

### Melody Too Long

Requests whose body exceeds `--max-melody-length` (default 1000 bytes)
are rejected with 400. Raise the limit, or split the melody into several
requests. Long melodies work with the CPAL backend; the FreeBSD speaker
driver has its own limits.

### Testing Without Hardware

Use a regular file as a mock device for testing:

```bash
# Start server with file device
./target/release/spkrd --device /tmp/test-speaker

# Send melody
curl -X PUT http://localhost:1111/play -d "cdefgab"

# Check result
cat /tmp/test-speaker
```


## See Also

- [Overview](README.md)
- [Installation instructions](INSTALL.md)
- [HTTP API documentation](API.md)
- [Client examples](examples/README.md)
- [Development notes](DEVELOPMENT.md)
