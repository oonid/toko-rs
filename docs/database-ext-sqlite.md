# SQLite Extension

## Overview

toko-rs supports SQLite as an optional compile-time backend via Cargo feature flag. PostgreSQL remains the default and recommended production backend. SQLite is useful for:

- **Local development** without Docker/PostgreSQL
- **Integration testing** against an in-memory database
- **Embedded/single-user deployments**

## Feature Flag

```bash
# PostgreSQL (default)
cargo build

# SQLite
cargo build --features sqlite --no-default-features
```

The binary is compiled for exactly one backend. There is no runtime backend switching.

### Cargo.toml

```toml
[features]
default = ["postgres"]
postgres = ["sqlx/postgres"]
sqlite = ["sqlx/sqlite"]
```

## Architecture

### Type Aliases (src/db.rs)

Only `src/db.rs` uses `#[cfg]` guards. All other files use generic type aliases:

| Alias | PostgreSQL (default) | SQLite |
|---|---|---|
| `DbPool` | `PgPool` | `SqlitePool` |
| `DbPoolOptions` | `PgPoolOptions` | `SqlitePoolOptions` |
| `DbDatabase` | `sqlx::Postgres` | `sqlx::Sqlite` |
| `DbTransaction<'a>` | `sqlx::Transaction<'a, Postgres>` | `sqlx::Transaction<'a, Sqlite>` |

### cfg Scope

`#[cfg]` guards touch only these locations in `src/db.rs`:

1. Type alias definitions (5 aliases)
2. `create_db()` — pool construction (SQLite uses `max_connections(1)` + `PRAGMA foreign_keys = ON`)
3. `run_migrations()` — selects `./migrations/` (PG) or `./migrations/sqlite/` (SQLite)
4. Error code constants — `unique_violation_code()`, `fk_violation_code()`, `not_null_violation_code()`

All 5 repository files, all route handlers, and all test files use `DbPool`/`DbTransaction` with zero cfg guards.

## SQL Compatibility

The same SQL queries work on both backends because:

| Feature | PostgreSQL | SQLite | Notes |
|---|---|---|---|
| Parameter placeholders | `$1, $2, ...` | `$1, $2, ...` | sqlx normalizes to `?` for SQLite |
| `RETURNING *` | Yes | Yes (3.35+) | Bundled libsqlite3-sys ships 3.39+ |
| `ON CONFLICT DO NOTHING` | Yes | Yes (3.24+) | Used in seed.rs (14 occurrences) |
| `CURRENT_TIMESTAMP` | Yes | Yes | Replaces PG-only `now()` |
| `JSONB` columns | Native | Stored as TEXT | Application-layer JSON; no queries use JSON operators |
| `BOOLEAN` | Native | INTEGER (0/1) | sqlx handles mapping |
| Partial unique indexes | Yes | Yes (3.8+) | `WHERE deleted_at IS NULL` |
| Cascading FKs | Yes | Yes (with PRAGMA) | `PRAGMA foreign_keys = ON` set on connect |
| `UPDATE ... RETURNING` | Yes | Yes (3.35+) | Used for `_sequences` display_id generation |

### SQL Changes for Portability

| Change | Files | Occurrences |
|---|---|---|
| `now()` → `CURRENT_TIMESTAMP` | product, cart, customer, order repos | 9 |

## Migration Differences

### PostgreSQL (`./migrations/`)

- `TIMESTAMPTZ` columns with `DEFAULT now()`
- `JSONB` columns
- `BOOLEAN` columns
- `BIGINT` for i64 columns
- `CHECK (status IN (...))` constraints
- `CREATE UNIQUE INDEX ... WHERE` for partial unique constraints
- Performance indexes with `WHERE deleted_at IS NULL` filters

### SQLite (`./migrations/sqlite/`)

- `DATETIME` columns with `DEFAULT CURRENT_TIMESTAMP`
- `TEXT` columns storing JSON
- `INTEGER` columns (0/1) for booleans
- `INTEGER` for i64 columns (SQLite integers are 64-bit)
- Same `CHECK` constraints (supported since 3.25+)
- Same partial unique indexes (supported since 3.8+)
- Same performance indexes

## Error Codes

Error detection uses backend-specific constraint codes, abstracted behind helper functions:

| Constraint | PG Code | SQLite Code | Helper |
|---|---|---|---|
| Unique violation | `23505` | `2067` | `db::is_unique_violation()` |
| FK violation | `23503` | `787` | `db::is_fk_violation()` |
| Not-null violation | `23502` | `1299` | `db::is_not_null_violation()` |

## Running

### Local Development (SQLite file)

```bash
cargo build --features sqlite --no-default-features
DATABASE_URL=sqlite:toko.db cargo run
```

### Testing (SQLite in-memory)

```bash
DATABASE_URL="sqlite::memory:" cargo test --features sqlite --no-default-features -- --test-threads=1
```

Or via Makefile:

```bash
make test-sqlite
```

### Full Suite (Both Backends)

```bash
make test-all
```

This runs `make test-pg` followed by `make test-sqlite`.

## Limitations

- **Single connection**: SQLite pool uses `max_connections(1)` for write safety. Not suitable for concurrent write workloads.
- **No `JSONB` operators**: JSON columns are stored as TEXT. Raw SQL queries using `->>`, `@>`, or other JSONB operators would need adaptation. toko-rs doesn't use these in P1.
- **No advisory locks**: PostgreSQL advisory locks (`pg_advisory_lock`) are not available. Used for idempotency in P2.
- **Database-level locking**: SQLite locks the entire database during writes. Under concurrent access, writes serialize automatically but may timeout.
- **Migration tooling**: `sqlx::migrate!()` is compile-time resolved. Each backend has its own migration directory. Schema changes must be applied to both.
