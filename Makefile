build:
	cargo build --release

install:
	cargo install --locked --path .

clean:
	cargo clean

assets: build
	@mkdir -p assets/images
	@for file in examples/*.dgmy; do \
		basename=$$(basename $$file .dgmy); \
		./target/release/dgmy $$file -o assets/images/$$basename.svg; \
	done

.PHONY: build install clean assets
