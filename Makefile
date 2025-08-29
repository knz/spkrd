# Makefile for spkrd - FreeBSD speaker device network server

# Configuration variables
DSTDIR ?= /usr/local
PROGRAM ?= spkrd
BUILD ?= release

# Build target selection
ifeq ($(BUILD),release)
    CARGO_FLAGS = --release
    TARGET_DIR = target/release
else
    CARGO_FLAGS = 
    TARGET_DIR = target/debug
endif

# Default target
.PHONY: all clean install

all:
	cargo build $(CARGO_FLAGS)

clean:
	cargo clean

install: all
	install -d $(DSTDIR)/bin
	install -m 755 $(TARGET_DIR)/$(PROGRAM) $(DSTDIR)/bin/$(PROGRAM)
	install -d $(DSTDIR)/etc/rc.d
	install -m 755 rc.d/$(PROGRAM) $(DSTDIR)/etc/rc.d/$(PROGRAM)