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
- `make install` - Build and install the client binary

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
./target/release/client --server http://server:8080 "t120l8cdefgab"

# Short form
./target/release/client -s http://192.168.1.100:8080 "cdefgab"

# Build and run with cargo
cargo run --bin client -- --server http://server:8080 "t120l8cdefgab"
```

#### Config File Usage

```bash
# Create config file with server URL
echo "http://server:8080" > ~/.spkrc

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
go run client.go http://server:8080 "cdefgab"

# Or build first
go build client.go
./client http://server:8080 "cdefgab"
```

## Example Melodies

- Simple scale: `"cdefgab"`
- With tempo: `"t120l4 c d e f g a b o5c"`
- Complex melody: `"t150l8 c d e f g f e d c p l4 g"`

For complete melody syntax, see the [FreeBSD speaker(4) manual](https://man.freebsd.org/cgi/man.cgi?query=speaker&apropos=0&sektion=0&manpath=FreeBSD+14.3-RELEASE+and+Ports&arch=default&format=html).