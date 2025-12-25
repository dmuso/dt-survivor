.PHONY: run build build-release check lint test clean

run:
	cargo run

build:
	cargo build

build-release:
	cargo build --release

check:
	cargo check

lint:
	cargo clippy

test:
	cargo test --quiet

clean:
	cargo clean
