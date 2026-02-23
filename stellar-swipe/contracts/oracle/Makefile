default: build

test:
	cargo test

build:
 feature/oracle-price-conversion
	soroban contract build
	@ls -lh ../../target/wasm32-unknown-unknown/release/*.wasm

fmt:
	cargo fmt --all

clean:
	cargo clean
=======
	cargo build --target wasm32-unknown-unknown --release

clean:
	cargo clean

fmt:
	cargo fmt

check:
	cargo check
 main
