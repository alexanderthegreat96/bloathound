NAME     := bloathound
BIN_DIR  := bin

WIN_X64   := x86_64-pc-windows-msvc
LINUX_X64 := x86_64-unknown-linux-musl
LINUX_ARM := aarch64-unknown-linux-musl
MAC_ARM   := aarch64-apple-darwin
MAC_X64   := x86_64-apple-darwin

HOST := $(shell uname -s)

ifeq ($(HOST),Darwin)
  HOST_BUILDABLE := macos linux linux-arm
else ifeq ($(HOST),Linux)
  HOST_BUILDABLE := linux linux-arm
else
  HOST_BUILDABLE := windows linux linux-arm
endif

.DEFAULT_GOAL := help
.PHONY: help all windows linux linux-arm macos targets check test fmt lint clean

help:
	@echo "$(NAME) build targets            (host: $(HOST))"
	@echo
	@echo "  make all         Build every platform this machine can produce:"
	@echo "                   $(HOST_BUILDABLE)"
	@echo "  make windows     Windows x86_64        -> $(BIN_DIR)/$(NAME)-windows-x86_64.exe"
	@echo "  make linux       Linux x86_64, static  -> $(BIN_DIR)/$(NAME)-linux-x86_64"
	@echo "  make linux-arm   Linux aarch64, static -> $(BIN_DIR)/$(NAME)-linux-aarch64"
	@echo "  make macos       macOS aarch64         -> $(BIN_DIR)/$(NAME)-macos-aarch64"
	@echo
	@echo "  make targets     Install the rustup targets the builds need"
	@echo "  make check       Clippy every platform, tests included"
	@echo "  make test        Run the test suite"
	@echo "  make fmt         Format the source"
	@echo "  make lint        Check formatting and clippy without changing anything"
	@echo "  make clean       Remove $(BIN_DIR)/ and the cargo target directory"

all: $(HOST_BUILDABLE)
	@echo
	@echo "Built into $(BIN_DIR)/:"
	@ls -lh $(BIN_DIR)

targets:
	rustup target add $(LINUX_X64) $(LINUX_ARM)
ifeq ($(HOST),Darwin)
	rustup target add $(MAC_ARM)
else ifneq ($(HOST),Linux)
	rustup target add $(WIN_X64)
endif

linux:
	cargo build --release --target $(LINUX_X64)
	@mkdir -p $(BIN_DIR)
	cp target/$(LINUX_X64)/release/$(NAME) $(BIN_DIR)/$(NAME)-linux-x86_64

linux-arm:
	cargo build --release --target $(LINUX_ARM)
	@mkdir -p $(BIN_DIR)
	cp target/$(LINUX_ARM)/release/$(NAME) $(BIN_DIR)/$(NAME)-linux-aarch64

windows:
ifeq ($(filter Darwin Linux,$(HOST)),)
	cargo build --release --target $(WIN_X64)
	@mkdir -p $(BIN_DIR)
	cp target/$(WIN_X64)/release/$(NAME).exe $(BIN_DIR)/$(NAME)-windows-x86_64.exe
else
	@echo "Windows binaries need the MSVC libraries and cannot be built on $(HOST)."
	@echo "Build this one on a Windows host, or on a windows runner in CI."
	@exit 1
endif

macos:
ifeq ($(HOST),Darwin)
	cargo build --release --target $(MAC_ARM)
	@mkdir -p $(BIN_DIR)
	cp target/$(MAC_ARM)/release/$(NAME) $(BIN_DIR)/$(NAME)-macos-aarch64
else
	@echo "macOS binaries cannot be cross-compiled from $(HOST)."
	@echo
	@echo "The linker needs the Apple SDK to resolve -framework Foundation,"
	@echo "-framework IOKit and -lobjc, which the trash crate pulls in."
	@echo
	@echo "Build this one on a Mac, or on a macos runner in CI."
	@echo "The code itself is still checked for macOS by 'make check'."
	@exit 1
endif

check:
	cargo clippy --all-targets -- -D warnings
	cargo clippy --all-targets --target $(LINUX_X64) -- -D warnings
	cargo clippy --all-targets --target $(MAC_ARM) -- -D warnings

test:
	cargo test

fmt:
	cargo fmt

lint:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings

clean:
	cargo clean
	rm -rf $(BIN_DIR)
