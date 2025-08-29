# Makefile for spkrd client examples

# Configuration variables
BUILD ?= release
PROGRAM ?= spkrc
DSTDIR ?= /usr/local/bin

# Internal variables
TARGET_DIR = target/$(BUILD)
BINARY_PATH = $(TARGET_DIR)/client

.PHONY: all clean install

all:
	cargo build --$(BUILD)

clean:
	cargo clean

install: all
	install -m 755 $(BINARY_PATH) $(DSTDIR)/$(PROGRAM)
	install -m 755 spkcmd $(DSTDIR)/spkcmd
