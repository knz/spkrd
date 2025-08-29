# spkrd Client Examples

This directory contains example client implementations for the spkrd speaker server.

## Files

- `client.rs` - Rust implementation of the spkrd client
- `client.go` - Go implementation of the spkrd client
- `Makefile` - Build and installation automation for the Rust client

## Building and Installing

The provided Makefile supports building and installing the Rust client with configurable options.

### Make Targets

- `make all` - Build the Rust client
- `make clean` - Remove build artifacts
- `make install` - Build and install the client binary and spkcmd utility

### Configuration Variables

All variables can be overridden on the command line:

- `BUILD` - Build mode (default: `release`)

  - `make BUILD=debug` - Build in debug mode
  - `make BUILD=release` - Build in release mode (default)

- `PROGRAM` - Installed binary name (default: `spkrc`)

  - `make PROGRAM=myclient install` - Install as 'myclient'

- `DSTDIR` - Installation directory (default: `/usr/local/bin`)
  - `make DSTDIR=/opt/bin install` - Install to /opt/bin

### Examples

```bash
# Build in release mode (default)
make

# Build in debug mode
make BUILD=debug

# Install with default name (spkrc) to /usr/local/bin
make install

# Install with custom name to custom directory
make PROGRAM=speaker-client DSTDIR=$HOME/.local/bin install

# Build debug version and install with custom name
make BUILD=debug PROGRAM=spkrc-debug install
```

## Requirements

- Rust and Cargo must be installed
- Installation may require appropriate permissions for the target directory

## Client Usage

### Rust Client

The Rust client supports flexible server configuration via command line options or a config file.

#### Command Line Usage

```bash
# Using --server option
./target/release/client --server http://server:1111 "t120l8cdefgab"

# Short form
./target/release/client -s http://192.168.1.100:1111 "cdefgab"

# Build and run with cargo
cargo run --bin client -- --server http://server:1111 "t120l8cdefgab"
```

#### Config File Usage

```bash
# Create config file with server URL
echo "http://server:1111" > ~/.spkrc

# Now you can run without --server option
./target/release/client "cdefgab"
```

#### Client Options

- `-s, --server <URL>` - Server URL (overrides config file)
- `<MELODY>` - Melody string to play (required)
- `-h, --help` - Show help message

#### Configuration Priority

1. Command line `--server` option (highest priority)
2. Config file `~/.spkrc` (fallback)
3. Error if neither is provided

### Go Client

```bash
# Basic usage
go run client.go http://server:1111 "cdefgab"

# Or build first
go build client.go
./client http://server:1111 "cdefgab"
```

## Example Melodies

- Simple scale: `"cdefgab"`
- With tempo: `"t120l4 c d e f g a b o5c"`
- Complex melody: `"t150l8 c d e f g f e d c p l4 g"`

For complete melody syntax, see the [FreeBSD speaker(4) manual](https://man.freebsd.org/cgi/man.cgi?query=speaker&apropos=0&sektion=0&manpath=FreeBSD+14.3-RELEASE+and+Ports&arch=default&format=html).

## Audio Feedback Utility

The `spkcmd` script provides audio feedback for command exit codes, optimized for use with the spkrd Rust client.

### Usage

Use `spkcmd` as a prefix to any command to get audio feedback based on the exit status:

```bash
# Add audio feedback to any command
spkcmd make test
spkcmd cargo build --release
spkcmd ls /nonexistent
```

### Audio Feedback

- **Success (exit 0)**: Pleasant ascending notes
- **Interrupted (exit 130)**: Silent (respects user cancellation)
- **Standard errors (exit 1-127)**: Low warning tone
- **Fatal errors (exit 128+)**: Urgent high tone

### Requirements

- The `spkrc` client must be installed and available in PATH
- Works best with a running spkrd server for immediate audio feedback

## Shell Integration

For automatic audio feedback on all command line operations, source the provided shell configuration files.

### Available Configurations

- `spkcmd-bash.sh` - Bash integration using function wrappers
- `spkcmd-zsh.sh` - Zsh integration using function wrappers and preexec

### Installation

Add one of these lines to your shell configuration file:

```bash
# For bash users - add to ~/.bashrc or ~/.bash_profile
source /usr/local/share/spkrd/examples/spkcmd-bash.sh

# For zsh users - add to ~/.zshrc
source /usr/local/share/spkrd/examples/spkcmd-zsh.sh
```

### Usage

Once loaded, audio feedback is automatically enabled for most commands:

```bash
# These commands will automatically get audio feedback
make test          # Success: pleasant ascending notes, Error: warning/error tones
cargo build        # Same audio feedback based on exit status
git commit -m "..."
npm install
```

### Control Functions

- `spkcmd_on` - Enable automatic audio feedback (default state)
- `spkcmd_off` - Disable automatic audio feedback

### Filtered Commands

The integration automatically excludes audio feedback for:

- **Built-in commands**: cd, pwd, echo, export, etc.
- **Fast commands**: ls, cat, grep, which, etc.
- **Interactive commands**: vim, ssh, less, man, etc.
- **Background jobs**: Any command ending with `&`
- **Already wrapped**: Commands already using spkcmd

This ensures audio feedback only occurs for meaningful operations while avoiding noise from quick utility commands.

### Requirements

- `spkrc` must be installed and available in PATH
- A running spkrd server for audio feedback
- Compatible shell (bash 4.0+ or zsh 5.0+)

### Dynamic Command Wrapping (Zsh Only)

The zsh integration uses dynamic command wrapping that automatically creates audio feedback wrappers for external commands on first use:

- **Priority commands** (make, git, cargo) are pre-wrapped for immediate availability
- **Additional commands** are wrapped dynamically when first encountered in interactive sessions
- **Interactive sessions only**: Dynamic wrapping only works in interactive zsh sessions, not in non-interactive contexts like `zsh -c` or scripts
- **First use delay**: New commands get audio feedback starting from their second use in the session

This approach provides intelligent audio feedback while maintaining performance and avoiding unnecessary wrapper creation for unused commands.
