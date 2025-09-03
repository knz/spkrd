# Makefile for spkrd client examples

# Configuration variables
PROGRAM ?= spkrc
DSTDIR ?= /usr/local/bin
PROFILE ?= release # can be 'dev' for debugging
TARGET_DIR = target/$(PROFILE)
BINARY_PATH = $(TARGET_DIR)/client
CARGO_FLAGS = --profile $(PROFILE)

.PHONY: all clean install

all: $(BINARY_PATH)

$(BINARY_PATH):
	cargo build $(CARGO_FLAGS)

clean:
	cargo clean

install: all
	install -m 755 $(BINARY_PATH) $(DSTDIR)/$(PROGRAM)
	install -m 755 spkcmd $(DSTDIR)/spkcmd
