.PHONY: dev test check lint fmt seed clean-db docker-up docker-down test-pg test-sqlite test-all test-e2e test-e2e-pg cov

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
	DATABASE_URL=postgres://postgres:postgres@localhost:5432/toko_test cargo test -- --test-threads=1

test-sqlite:
	DATABASE_URL="sqlite::memory:" cargo test --features sqlite --no-default-features -- --test-threads=1

test-all:
	$(MAKE) test-pg
	$(MAKE) test-sqlite

test-e2e:
	E2E_DATABASE_URL=postgres://postgres:postgres@localhost:5432/toko_e2e cargo test --test e2e -- --test-threads=1

test-e2e-pg:
	DATABASE_URL=postgres://postgres:postgres@localhost:5432/toko_test E2E_DATABASE_URL=postgres://postgres:postgres@localhost:5432/toko_e2e cargo test -- --test-threads=1

cov:
	cargo llvm-cov --summary-only
