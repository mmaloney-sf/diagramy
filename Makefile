build:
	cargo build --release

install:
	cargo install --locked --path .

.PHONY: build install
