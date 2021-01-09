# path to wasm-bindgen binary *built from source*
PKG_DIR=pkg
FLAGS=--release
build:
	echo "cross compiling to .wasm and generating bindings"
	rm -rf pkg
	# see .cargo/config.toml for all flags used
	RUSTFLAGS=' -C target-feature=+atomics,+bulk-memory' wasm-pack --verbose build --target no-modules --out-dir $(PKG_DIR) --out-name beh

simd:
	RUSTFLAGS=' -C target-feature=+atomics,+simd128,+bulk-memory -Cno-vectorize-loops -Cno-vectorize-slp -Copt-level=z -Clinker-flavor=em' \
	cargo build --target wasm32-unknown-unknown -Z build-std=std,panic_abort
	echo "Generating bindings"
	rm -rf pkg
	$(WASM_BINDGEN) ./target/wasm32-unknown-unknown/debug/beh.wasm  --out-dir $(PKG_DIR) --target no-modules

serve:
	python3 -m http.server

deploy:
	scp -r index.html  *.js pkg narekg@cycles.cs.princeton.edu:/u/narekg/public_html/wasm_rayon/

deployv:
	scp -r index.html  *.js *.php pkg "${MY_WEB_SERVER}"
