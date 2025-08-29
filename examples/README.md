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