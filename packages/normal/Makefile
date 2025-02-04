default: all

all: lint build test

test:
	cargo test

build:
	cargo build --target wasm32-unknown-unknown --release --profile release-with-logs

lint: fmt clippy

fmt:
	cargo fmt --all

clippy: build
	cargo clippy --all-targets -- -D warnings

clean:
	cargo clean
