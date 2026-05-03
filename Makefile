# Makefile for spkrd - speaker device network server
#
# Targets: all, clean, install
# Variables: DSTDIR (default /usr/local), PROGRAM, PROFILE
#
# install auto-detects the OS via uname(1) and installs either the FreeBSD
# rc.d script or the systemd unit file accordingly. Shell conditionals are
# used in the recipe (rather than make-level ifeq/.if directives) so that
# this Makefile remains compatible with both GNU make and BSD make.

# Configuration variables
DSTDIR ?= /usr/local
PROGRAM ?= spkrd
# PROFILE can be 'dev' for debugging
PROFILE ?= release
TARGET_DIR = target/$(PROFILE)
BINARY_PATH = $(TARGET_DIR)/$(PROGRAM)
CARGO_FLAGS = --profile $(PROFILE)

# Default target
.PHONY: all clean install

all: $(BINARY_PATH)

$(BINARY_PATH):
	cargo build $(CARGO_FLAGS)

clean:
	cargo clean

install: $(BINARY_PATH)
	install -d $(DSTDIR)/bin
	install -m 755 $(TARGET_DIR)/$(PROGRAM) $(DSTDIR)/bin/$(PROGRAM)
	@OS=$$(uname -s); \
	if [ "$$OS" = "Linux" ]; then \
		install -d $(DSTDIR)/lib/systemd/system; \
		install -m 644 systemd/$(PROGRAM).service $(DSTDIR)/lib/systemd/system/$(PROGRAM).service; \
		echo ""; \
		echo "Systemd unit installed to $(DSTDIR)/lib/systemd/system/$(PROGRAM).service"; \
		echo "To enable and start the service, run:"; \
		echo "  systemctl daemon-reload"; \
		echo "  systemctl enable $(PROGRAM)"; \
		echo "  systemctl start $(PROGRAM)"; \
	elif [ "$$OS" = "FreeBSD" ]; then \
		install -d $(DSTDIR)/etc/rc.d; \
		install -m 755 rc.d/$(PROGRAM) $(DSTDIR)/etc/rc.d/$(PROGRAM); \
	else \
		echo "Unknown operating system. Please set up the service to start automatically on boot."; \
	fi
