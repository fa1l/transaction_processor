COVERAGE ?= 85

init:
	rustup component add clippy
	cargo install cargo-audit typos-cli cargo-tarpaulin

pretty:
	cargo fmt

lint:
	cargo fmt -- --check
	cargo check --all-targets --all-features
	cargo clippy --all-targets --all-features -- -D warnings
	cargo audit
	typos .

plint:	pretty lint

tests: 
	cargo tarpaulin --fail-under $(COVERAGE) --exclude-files src/errors.rs

codecov:
	cargo tarpaulin --out Html --exclude-files src/errors.rs
