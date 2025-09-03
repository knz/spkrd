# Makefile for spkrd - FreeBSD speaker device network server

# Configuration variables
DSTDIR ?= /usr/local
PROGRAM ?= spkrd
PROFILE ?= release # can be 'dev' for debugging
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

install: all
	install -d $(DSTDIR)/bin
	install -m 755 $(TARGET_DIR)/$(PROGRAM) $(DSTDIR)/bin/$(PROGRAM)
	install -d $(DSTDIR)/etc/rc.d
	install -m 755 rc.d/$(PROGRAM) $(DSTDIR)/etc/rc.d/$(PROGRAM)
