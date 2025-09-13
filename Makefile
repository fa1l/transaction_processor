init:
	rustup component add clippy
	cargo install cargo-audit typos-cli

pretty:
	cargo fmt

lint:
	cargo fmt -- --check
	cargo check --all-targets --all-features
	cargo clippy --all-targets --all-features -- -D warnings
	cargo audit
	typos check .

plint:	pretty lint
