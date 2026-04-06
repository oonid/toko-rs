.PHONY: dev test check lint fmt seed clean-db

dev:
	cargo run

test:
	cargo test

check:
	cargo check

lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

seed:
	cargo run -- --seed

clean-db:
	rm -f toko.db
