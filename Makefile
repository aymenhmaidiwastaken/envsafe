.PHONY: all build release test fmt clippy clean install

all: fmt clippy test build

build:
	cargo build

release:
	cargo build --release

test:
	cargo test --all-features

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

clean:
	cargo clean

install:
	cargo install --path .
