default: build

test:
	cargo test

build:
	soroban contract build
	@ls -lh ../../target/wasm32-unknown-unknown/release/*.wasm

fmt:
	cargo fmt --all

clean:
	cargo clean
