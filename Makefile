# path to wasm-bindgen binary *built from source*
WASM_BINDGEN=../wasm-bindgen/target/debug/wasm-bindgen
PKG_DIR=pkg
FLAGS=--release
build:
	echo "cross compiling to .wasm"
	# these rust flags are necessary to make sure:
	# *bulk-memory: .wasm module and .wasm memory can be passed between threads
	# *atomics: the invariant that threads only use atomic instractions with shared memory is preserved
	#RUSTFLAGS='-C target-feature=+atomics,+bulk-memory' cargo build --target wasm32-unknown-unknown --release -Z build-std=std,panic_abort
	RUSTFLAGS=' -C target-feature=+atomics,+bulk-memory' \
	cargo build --target wasm32-unknown-unknown $(FLAGS) -Z build-std=std,panic_abort
	echo "Generating bindings"
	rm -r pkg
	$(WASM_BINDGEN) ./target/wasm32-unknown-unknown/release/beh.wasm  --out-dir $(PKG_DIR) --target no-modules

simd:
	RUSTFLAGS=' -C target-feature=+atomics,+simd128,+bulk-memory -Cno-vectorize-loops -Cno-vectorize-slp -Copt-level=z -Clinker-flavor=em' \
	cargo build --target wasm32-unknown-unknown -Z build-std=std,panic_abort
	echo "Generating bindings"
	rm -r pkg
	$(WASM_BINDGEN) ./target/wasm32-unknown-unknown/debug/beh.wasm  --out-dir $(PKG_DIR) --target no-modules

serve:
	python -m http.server

deploy:
	scp -r index.html  *.js pkg narekg@cycles.cs.princeton.edu:/u/narekg/public_html/wasm_rayon/