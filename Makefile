.PHONY: dev test check lint fmt seed clean-db docker-up docker-down test-pg cov

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

docker-up:
	docker compose up -d

docker-down:
	docker compose down

test-pg:
	DATABASE_URL=postgres://postgres:postgres@localhost:5432/toko cargo test

cov:
	cargo llvm-cov --summary-only
