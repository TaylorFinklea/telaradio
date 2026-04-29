# Telaradio — top-level Make targets that orchestrate the Rust workspace
# and the Swift package.
#
# The macOS deployment target is pinned to 13.0 so the Rust static lib
# matches the SwiftPM package's `platforms: [.macOS(.v13)]` declaration.
# Without this, you get "object file was built for newer 'macOS' version"
# linker warnings.

export MACOSX_DEPLOYMENT_TARGET = 13.0

CARGO        ?= cargo
SWIFT        ?= swift
RUST_PROFILE ?= debug
SWIFT_DIR     = apple/Telaradio

.PHONY: help all ffi swift app app-run test lint fmt clean

help:
	@echo "Telaradio Makefile targets:"
	@echo "  make ffi      — cargo build telaradio-ffi (regenerates the C header too)"
	@echo "  make swift    — swift build (links the Rust static lib)"
	@echo "  make app      — make ffi && make swift"
	@echo "  make app-run  — make app && launch the Telaradio binary"
	@echo "  make test     — cargo test --workspace"
	@echo "  make lint     — cargo clippy --all-targets -- -D warnings"
	@echo "  make fmt      — cargo fmt"
	@echo "  make clean    — remove cargo and swift build artifacts"

all: app

ffi:
	$(CARGO) build -p telaradio-ffi

ffi-release:
	$(CARGO) build -p telaradio-ffi --release

swift: ffi
	cd $(SWIFT_DIR) && $(SWIFT) build

app: swift

app-run: app
	cd $(SWIFT_DIR) && $(SWIFT) run

test:
	$(CARGO) test --workspace

lint:
	$(CARGO) clippy --all-targets -- -D warnings

fmt:
	$(CARGO) fmt

clean:
	$(CARGO) clean
	cd $(SWIFT_DIR) && rm -rf .build
