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

web:
	wasm-pack build --target web --out-dir web/dist -- --no-default-features --features wasm-bindgen

.PHONY: build install clean assets web
