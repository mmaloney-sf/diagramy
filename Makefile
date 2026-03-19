build:
	cargo build --release --features lsp

install:
	cargo install --locked --path .

test:
	cargo test

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
	./web/build-examples.sh

serve-web:
	cd web
	python3 -m http.server 8002

.PHONY: build install clean assets web serve-web test
