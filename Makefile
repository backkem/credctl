.PHONY: all check fmt clippy test build build-gui build-tui build-all run run-tui clean

all: check fmt clippy test

check:
	cargo check --no-default-features --features gui
	cargo check --no-default-features --features tui
	cargo check --no-default-features --features "gui tui"

fmt:
	cargo fmt --all --check

clippy:
	cargo clippy --no-default-features --features gui -- -D warnings
	cargo clippy --no-default-features --features tui -- -D warnings
	cargo clippy --no-default-features --features "gui tui" -- -D warnings

test:
	cargo test --no-default-features --features gui
	cargo test --no-default-features --features tui
	cargo test --no-default-features --features "gui tui"

# Default build: GUI (xilem). Same as `make build-gui`.
build: build-gui

build-gui:
	cargo build --release --no-default-features --features gui

# TUI-only build: no xilem, no fontconfig, no wgpu.
build-tui:
	cargo build --release --no-default-features --features tui

# Both editors in one binary.
build-all:
	cargo build --release --no-default-features --features "gui tui"

run:
	cargo run --release --no-default-features --features gui --

run-tui:
	cargo run --release --no-default-features --features tui -- --editor tui

clean:
	cargo clean
