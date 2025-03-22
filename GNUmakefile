SHELL := /usr/bin/env bash
.SHELLFLAGS := -eu -o pipefail -c

# Set the default goal if "make" is run without arguments
.DEFAULT_GOAL := help

# Delete any targets that failed to force them to be rebuilt on the next run
.DELETE_ON_ERROR:

# Disable built-in C/Lex/Yacc rules, and warn if referencing undefined vars
MAKEFLAGS += --warn-undefined-variables
MAKEFLAGS += --no-builtin-rules

######################################################

VPATH = src
SRC = $(wildcard *.rs)
VERSION = $(shell cat Cargo.toml | sed -n 's/^version = "\(.*\)"$$/\1/p')

# Assumes a macOS host.
target/aarch64-apple-darwin/release/tlafmt: $(SRC)
	cargo build --release --target aarch64-apple-darwin

target/x86_64-unknown-linux-musl/release/tlafmt: $(SRC)
	cross build --release --target x86_64-unknown-linux-musl

target/x86_64-pc-windows-gnu/release/tlafmt.exe: $(SRC)
	cross build --release --target x86_64-pc-windows-gnu

# Windows-specific rule due to the trailing .exe
release/tlafmt_v$(VERSION)_x86_64-pc-windows-gnu.exe: target/x86_64-pc-windows-gnu/release/tlafmt.exe
	@-mkdir -p $(dir $@)
	cp $^ $@

release/tlafmt_v$(VERSION)_%: target/%/release/tlafmt
	@-mkdir -p $(dir $@)
	cp $^ $@

%.sha256sum: %
	sha256sum "$^" > $@

%.tar.gz: %
	tar cv "$^" | gzip --best > "$@"
	-rm $^

%.zip: %
	zip "$@" $^
	-rm $^

#? release: generate release binaries for Linux and macOS
.PHONY: release
release: release/tlafmt_v$(VERSION)_x86_64-unknown-linux-musl.tar.gz \
	release/tlafmt_v$(VERSION)_x86_64-unknown-linux-musl.tar.gz.sha256sum \
	release/tlafmt_v$(VERSION)_x86_64-pc-windows-gnu.exe.tar.gz \
	release/tlafmt_v$(VERSION)_x86_64-pc-windows-gnu.exe.tar.gz.sha256sum \
	release/tlafmt_v$(VERSION)_aarch64-apple-darwin.zip \
	release/tlafmt_v$(VERSION)_aarch64-apple-darwin.zip.sha256sum

#? clean: remove any generated files
.PHONY: clean
clean:
	-rm -rf release
	cargo clean

#? help: prints this help message
.PHONY: help
help:
	@echo "Usage:"
	@sed -n 's/^#?//p' ${MAKEFILE_LIST} | column -t -s ':' |  sed -e 's/^/ /'
