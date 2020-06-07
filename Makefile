# path to wasm-bindgen binary *built from source*
WASM_BINDGEN=../wasm-bindgen/target/debug/wasm-bindgen
PKG_DIR=pkg
build:
	echo "cross compiling to .wasm"
	# these rust flags are necessary to make sure:
	# *bulk-memory: .wasm module and .wasm memory can be passed between threads
	# *atomics: the invariant that threads only use atomic instractions with shared memory is preserved
	#RUSTFLAGS='-C target-feature=+atomics,+bulk-memory' cargo build --target wasm32-unknown-unknown --release -Z build-std=std,panic_abort
	RUSTFLAGS=' -C target-feature=+atomics,+bulk-memory' \
	cargo build --target wasm32-unknown-unknown --release -Z build-std=std,panic_abort
	echo "Generating bindings"
	rm -r pkg
	$(WASM_BINDGEN) ./target/wasm32-unknown-unknown/release/beh.wasm  --out-dir $(PKG_DIR) --target no-modules

serve:
	python -m http.server