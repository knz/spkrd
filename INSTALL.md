# Installing SPKRD

Build and installation instructions for the SPKRD server. For what the
server does, see [README.md](README.md); for running it, see
[USAGE.md](USAGE.md); for the HTTP API, see [API.md](API.md); for working
on the code, see [DEVELOPMENT.md](DEVELOPMENT.md).

## Prerequisites

- Rust 1.70+ and Cargo
- For the `freebsd-speaker` backend: FreeBSD with a `/dev/speaker` device
- For the `cpal` backend: any CPAL-supported host — Linux (ALSA,
  PipeWire, PulseAudio, JACK), macOS (CoreAudio), Windows (WASAPI)
- Go 1.19+, only if you want to build the Go example client

## Building

```bash
git clone <repository-url>
cd spkrd
make
```

`make` builds a release binary at `target/release/spkrd` with the
default feature set (which includes CPAL). It also auto-detects which
optional audio backends your system can support — see
[Build features](#build-features) below.

To drive Cargo directly instead, skipping the auto-detection:

```bash
cargo build --release
```

Use `PROFILE=dev` for a debug build:

```bash
make PROFILE=dev
```

## Build features

| Feature | Default | What it adds | System library required |
|---------|---------|--------------|-------------------------|
| `cpal` | yes | CPAL audio backend (ALSA on Linux) | — |
| `jack` | no | JACK host for `--cpal-host JACK` | `libjack` |
| `pulseaudio` | no | PulseAudio host for `--cpal-host PulseAudio` | `libpulse` |
| `pipewire` | no | PipeWire host for `--cpal-host PipeWire` | `libpipewire`, `libclang` (bindgen) |

### Feature auto-detection via the Makefile

The Makefile's `FEATURES` variable is empty by default. When it is empty,
the build recipe probes for each optional backend with `pkg-config` and
enables the corresponding feature for every library it finds:

| Probed package | Feature enabled |
|----------------|-----------------|
| `jack` | `jack` |
| `libpulse` | `pulseaudio` |
| `libpipewire-0.3` | `pipewire` |

Set `FEATURES` explicitly to force a specific set and skip the probing:

```bash
# Force JACK and PulseAudio, regardless of what pkg-config finds
make FEATURES=jack,pulseaudio
```

### Selecting features with Cargo directly

```bash
# Default build: CPAL backend with ALSA
cargo build --release

# With JACK and PulseAudio support
cargo build --release --features jack,pulseaudio

# With all Linux audio backends
cargo build --release --features jack,pulseaudio,pipewire

# FreeBSD: kernel backend only (removes the cpal dependency entirely)
cargo build --release --no-default-features
```

### Runtime host selection

When the `pipewire` or `pulseaudio` features are enabled, cpal's default
host selection picks the best available backend automatically at runtime:
PipeWire (if running) → PulseAudio (if running) → ALSA. JACK is never
selected automatically; it must be requested explicitly via
`--cpal-host JACK`.

When built without the `cpal` feature, `--output=auto` fails at startup
if the configured device path does not exist, rather than silently
falling back to a CPAL backend that was not compiled in.

## System-wide installation

```bash
# Install to system directories (default prefix: /usr/local)
make install

# Or install to a custom prefix
make install DSTDIR=/opt

# Or install under a custom program name
make install PROGRAM=my-spkrd
```

`make install` always installs the binary to `$(DSTDIR)/bin/spkrd`, then
detects the operating system with `uname -s` and installs the matching
service integration.

### FreeBSD

An rc.d script is installed to `$(DSTDIR)/etc/rc.d/spkrd`.

Enable the service by adding to `/etc/rc.conf`:

```sh
spkrd_enable="YES"
spkrd_flags="--port 1111 --device /dev/speaker --retry-timeout 30"
```

The rc.d script always passes `--daemon` and `--pidfile`; put any other
flags in `spkrd_flags`. See
[Command line options](USAGE.md#command-line-options) for the full list.

Some `spkrd_flags` examples:

```sh
# Custom port
spkrd_flags="--port 3000"

# Listen on localhost only (IPv4)
spkrd_flags="--bind 127.0.0.1"

# Listen on localhost only, IPv6, custom port
spkrd_flags="--bind [::1]:9000"

# Different device and port
spkrd_flags="--device /tmp/test-speaker --port 9000"

# Extended retry timeout
spkrd_flags="--retry-timeout 60"

# Enable debug logging (shows client requests in logs)
spkrd_flags="--debug"

# Custom PID file location for non-root execution
spkrd_flags="--pidfile /tmp/spkrd.pid"
```

Managing the service:

```bash
service spkrd start
service spkrd stop
service spkrd restart
service spkrd status
```

### Linux

A **systemd user unit** is installed to
`$(DSTDIR)/lib/systemd/user/spkrd.service`.

It is a user unit rather than a system unit deliberately: the CPAL
backend needs access to the per-user PulseAudio / PipeWire socket, which
a system-level service does not have. Each user runs their own instance.

Enable and start it for the current user:

```bash
systemctl --user daemon-reload
systemctl --user enable spkrd
systemctl --user start spkrd
```

By default a user service runs only while that user is logged in. To
start it at boot without a login session:

```bash
loginctl enable-linger $USER
```

The shipped unit runs:

```
ExecStart=/usr/local/bin/spkrd --port 1111 --cpal-host=PulseAudio
```

To change the flags without editing the installed unit, create a drop-in:

```bash
systemctl --user edit spkrd
```

and add:

```ini
[Service]
ExecStart=
ExecStart=/usr/local/bin/spkrd --port 3000 --debug
```

The empty `ExecStart=` is required — it clears the original value before
the replacement is applied. See
[Command line options](USAGE.md#command-line-options) for the full flag
list.

Logs are captured by journald:

```bash
journalctl --user -u spkrd -f
```

### Other operating systems

`make install` installs the binary and prints a notice that no service
integration was installed. Set up autostart with whatever mechanism your
system provides.

## Installing the example clients

The example clients in `examples/` have their own build and installation
process. See [examples/README.md](examples/README.md).

```bash
cd examples
make install     # builds the Rust client, installs it as spkrc, plus spkcmd
```

## Verifying the installation

Check that the binary runs and reports the expected configuration:

```bash
spkrd --version
spkrd --help
```

### Without sound hardware

Point `--device` at a regular file. Since the path exists, `--output=auto`
resolves to the `freebsd-speaker` backend, which simply writes the melody
to that file — no audio is produced.

```bash
touch /tmp/test-speaker
spkrd --device /tmp/test-speaker --debug
```

From another terminal:

```bash
curl -X PUT http://localhost:1111/play -d "cdefgab"
cat /tmp/test-speaker      # should contain: cdefgab
```

### With audio output

Point `--device` at a path that does not exist, so `--output=auto` falls
back to the `cpal` backend, or select it explicitly:

```bash
spkrd --output cpal --debug
```

Then send a melody as above; you should hear it played.

On startup the server logs its full configuration and one line per bound
listener:

```
[INFO  spkrd] Starting spkrd: bind=[0.0.0.0:1111, [::]:1111], retry_timeout=30s, ...
[INFO  spkrd::server] Server listening on 0.0.0.0:1111
[INFO  spkrd::server] Server listening on [::]:1111
```

## Uninstalling

There is no `uninstall` target. Remove the files that `make install`
created:

```bash
# All systems
rm /usr/local/bin/spkrd

# FreeBSD
rm /usr/local/etc/rc.d/spkrd

# Linux
systemctl --user disable --now spkrd
rm /usr/local/lib/systemd/user/spkrd.service
systemctl --user daemon-reload
```

Adjust the paths if you installed with a custom `DSTDIR` or `PROGRAM`.
