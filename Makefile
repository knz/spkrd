# Makefile for spkrd - speaker device network server
#
# Targets: all, clean, install
# Variables: DSTDIR (default /usr/local), PROGRAM, PROFILE
#
# install auto-detects the OS via uname(1): on Linux it installs a systemd
# user unit (lib/systemd/user/) so the service runs in the user session and
# has access to PulseAudio/PipeWire; on FreeBSD it installs the rc.d script.
# Shell conditionals are used in the recipe (rather than make-level ifeq/.if
# directives) so that this Makefile remains compatible with GNU make and BSD make.

# Configuration variables
DSTDIR ?= /usr/local
PROGRAM ?= spkrd
# PROFILE can be 'dev' for debugging
PROFILE ?= release
TARGET_DIR = target/$(PROFILE)
BINARY_PATH = $(TARGET_DIR)/$(PROGRAM)
CARGO_FLAGS = --profile $(PROFILE)
# Optional Cargo features to enable. When empty (the default) the build
# recipe auto-detects jack/pulseaudio/pipewire via pkg-config and enables
# each feature whose system library is present. Override to force a specific
# set or to disable auto-detection: make FEATURES=jack,pulseaudio
FEATURES ?=

# Default target
.PHONY: all clean install

all: $(BINARY_PATH)

$(BINARY_PATH):
	@_F="$(FEATURES)"; \
	if [ -z "$$_F" ]; then \
		pkg-config --exists jack 2>/dev/null          && _F="$${_F:+$$_F,}jack"        || true; \
		pkg-config --exists libpulse 2>/dev/null       && _F="$${_F:+$$_F,}pulseaudio"  || true; \
		pkg-config --exists libpipewire-0.3 2>/dev/null && _F="$${_F:+$$_F,}pipewire"   || true; \
	fi; \
	FLAGS="$(CARGO_FLAGS)$${_F:+ --features $$_F}"; \
	echo "cargo build $$FLAGS"; \
	cargo build $$FLAGS

clean:
	cargo clean

install: $(BINARY_PATH)
	install -d $(DSTDIR)/bin
	install -m 755 $(TARGET_DIR)/$(PROGRAM) $(DSTDIR)/bin/$(PROGRAM)
	@OS=$$(uname -s); \
	if [ "$$OS" = "Linux" ]; then \
		install -d $(DSTDIR)/lib/systemd/user; \
		install -m 644 systemd/$(PROGRAM).service $(DSTDIR)/lib/systemd/user/$(PROGRAM).service; \
		echo ""; \
		echo "Systemd user unit installed to $(DSTDIR)/lib/systemd/user/$(PROGRAM).service"; \
		echo "To enable and start for the current user:"; \
		echo "  systemctl --user daemon-reload"; \
		echo "  systemctl --user enable $(PROGRAM)"; \
		echo "  systemctl --user start $(PROGRAM)"; \
		echo "To auto-start on boot without login:"; \
		echo "  loginctl enable-linger \$$USER"; \
	elif [ "$$OS" = "FreeBSD" ]; then \
		install -d $(DSTDIR)/etc/rc.d; \
		install -m 755 rc.d/$(PROGRAM) $(DSTDIR)/etc/rc.d/$(PROGRAM); \
	else \
		echo "Unknown operating system. Please set up the service to start automatically on boot."; \
	fi
