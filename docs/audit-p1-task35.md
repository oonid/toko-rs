# Task 35: Discovery Audit — Post-T34 Compatibility Sweep (Schema, Shape, Doc Drift)

**Date**: 2026-05-18 (consolidated rev. — merges original audit and cross-evaluation)
**Medusa vendor**: `40a60e85b1` (v2.15.2 / develop)
**Scope**: Fresh 6-dimension audit of toko-rs against Medusa v2 vendor, post Task 34 (fulfillment + payment capture). Targets findings not already in `audit-master-checklist.md` (B-1…B-32, S-1…S-37, V-1…V-12, E-1…E-12, D-1…D-35, L-1…L-17, C-1…C-4 — 148 prior fixes).
**Status**: 12 of 15 actionable findings Applied on disk; 3 Deferred to P2 (X-15, V-13, V-14); 1 reclassified as intentional divergence K-14 (formerly D-36); 2 spec-modification gaps still open (Mod 1c / Mod 1d — phone-constraint message discrimination).

This document supersedes the previous `audit-p1-task35-eval.md` cross-evaluation. Findings, the spec alignment, and verification against on-disk source are unified here.

## Methodology

| Source | Compared Against |
|--------|------------------|
| `src/order/{models,repository,routes,types}.rs` | `vendor/medusa/packages/medusa/src/api/store/orders/`, `packages/medusa/src/api/admin/orders/` |
| `src/cart/{models,repository,routes,types}.rs` | `vendor/medusa/packages/medusa/src/api/store/carts/` |
| `src/customer/{models,repository,routes,types}.rs` | `vendor/medusa/packages/modules/customer/src/models/customer.ts`, `packages/medusa/src/api/{admin,store}/customers/` |
| `src/product/{models,repository,routes,types}.rs` | `vendor/medusa/packages/medusa/src/api/admin/products/`, `packages/medusa/src/api/store/products/` |
| `src/payment/{models,repository}.rs`, `src/invoice/*` | `vendor/medusa/packages/modules/payment/`, no Medusa invoice (toko-rs extension) |
| `src/{error,extract,db,types,config,lib}.rs` | Medusa error-handler patterns |
| `migrations/*.sql` + `migrations/sqlite/*.sql` (14 files) | Medusa MikroORM model definitions in `packages/modules/*/src/models/` |
| `tests/*.rs` + `tests/common/mod.rs` | — (isolation, harness) |
| `docs/p1_additions.md`, `README.md` | Re-checked against actual `vendor/medusa` route files |
| `specs/store-modification.md` | sapa-rs integration plan (Mod 1–4) |

Cross-referenced every finding against `audit-master-checklist.md` to exclude already-fixed items, and against `specs/store-modification.md` to reconcile intentional divergences.

---

## Build & Quality Gates

| Gate | Result | Detail |
|------|--------|--------|
| `cargo clippy -- -D warnings` | PASS | No issues found |
| `cargo fmt --check` | PASS (now) | At audit time: 703 lines of diff across `src/invoice/routes.rs`, `src/lib.rs`, `src/order/{models,repository,routes}.rs`, etc. — code committed after T34 was not run through `cargo fmt`. Subsequently fixed (C-6). |
| `cargo test -- --test-threads=1` (PG) | **260 PASS** on second run; **flake observed** on first run (1 failure in `test_admin_update_variant_sku_uniqueness` — see BUG-5) |
| Test count | 260 (+11 over T34's 249) |

---

## Findings

### HIGH Severity

#### HIGH-1 (S-38): `OrderSummary` is missing the `credit_line_total` field — Medusa's `OrderSummaryDTO` requires it

**Severity**: HIGH (response-shape regression vs Medusa contract)
**File**: `src/order/models.rs:5-15`
**Medusa**: `vendor/medusa/packages/core/types/src/order/common.ts:56-74` defines `OrderSummaryDTO` with 8 fields: `pending_difference`, `current_order_total`, `original_order_total`, `transaction_total`, `paid_total`, `refunded_total`, `credit_line_total`, `accounting_total`.

S-20 added `credit_line_total` at the **top-level** of `OrderWithItems`, but Medusa also requires it **inside** `summary` (`OrderSummaryDTO.credit_line_total`). A Medusa frontend reading `order.summary.credit_line_total` will get `undefined`.

**Impact**: Frontend logic that branches on credit balance (e.g., "you have $X store credit applied to this order") breaks. P1 value is always 0, but the key must exist.

**Fix**: Add `credit_line_total: i64` to `OrderSummary`, set to 0 in `from_items()` next to `refunded_total`.

**Status**: Applied — verified at `src/order/models.rs:13` (`pub credit_line_total: i64`) and zero-initialised at line 246 in `from_items()`.

---

#### HIGH-2 (S-39): Cart `CartWithItems` missing `item_discount_total` — Medusa exposes it in `defaultStoreCartFields`

**Severity**: HIGH
**File**: `src/cart/models.rs:92-126` (`CartWithItems`)
**Medusa**: `vendor/medusa/packages/medusa/src/api/store/carts/query-config.ts:23` — `item_discount_total` is in the default field list returned from `GET /store/carts/:id`.

toko-rs exposes `discount_total`, `discount_subtotal`, `discount_tax_total`, `shipping_discount_total`, `original_shipping_discount_total` (S-20) but never added `item_discount_total`.

**Impact**: Medusa SDK clients reading `cart.item_discount_total` get `undefined`. Cosmetic in P1 (value is always 0 with no promo module), but key absence still surfaces as a missing-field assertion in shape contract tests.

**Fix**: Add `item_discount_total: i64` to `CartWithItems`, default 0 in `from_items()`. Mirror to `OrderWithItems` for parity — Medusa's order shape includes it too via the `item_*` family.

**Status**: Applied — verified at `src/cart/models.rs:123` and `src/order/models.rs:141`; both initialised to 0 in their respective `from_items()` paths.

---

#### HIGH-3 (Doc-1): `docs/p1_additions.md` misclassifies `POST /admin/orders/:id/complete` as a toko-rs "addition"

**Severity**: HIGH (documentation drift — visible in the only file new users read)
**Files**: `docs/p1_additions.md` sections 1 and 2

`docs/p1_additions.md` section 2 ("toko-rs Additions") originally classified these endpoints as "not in Medusa v2":

- `POST /admin/orders/:id/complete` — but Medusa **does** have `vendor/medusa/packages/medusa/src/api/admin/orders/[id]/complete/route.ts`
- `POST /admin/orders/:id/cancel` — Medusa **does** have `vendor/medusa/packages/medusa/src/api/admin/orders/[id]/cancel/route.ts`

The text "Medusa v2 uses workflow-based order completion … there is no direct `/orders/:id/complete` action" was factually wrong. These are **compliant endpoints** (with simplified semantics), not additions.

**Impact**: Misleading compatibility narrative. Belongs in section 1 "Compliant Endpoints" with a note that toko-rs uses simpler state transitions vs Medusa's workflow engine.

**Fix**: Move `complete` and `cancel` from `docs/p1_additions.md` section 2 to section 1 ("Admin: Orders (partial)") with a "simplified state-machine semantics" note. `fulfill`, `ship`, `capture-payment` remain Decision-22/23 simplifications and stay in section 2.

**Status**: Applied — `docs/p1_additions.md` section 1 now lists `POST /admin/orders/:id/cancel` and `POST /admin/orders/:id/complete` under "Admin: Orders (partial)" with the simplified-state-machine note; section 2 only retains `fulfill`, `ship`, and `capture-payment`.

---

#### Doc-2: README.md test count was stale ("259 tests")

**Severity**: LOW (documentation)
**File**: `README.md:271`

Reality: `cargo test` reports **260 passed** (T35 measurement), not 259. Endpoint count ("43 endpoint methods across 6 domain modules") and the "7 migrations, 14 tables" claim are correct.

**Status**: Applied — README.md line 271 updated to "260 tests".

---

### MEDIUM Severity

#### MEDIUM-1 (C-6): `cargo fmt` not run on T34 commit — 703 lines of formatting drift

**Severity**: MEDIUM (CI hygiene; would fail any `cargo fmt --check` gate)
**Files**: `src/invoice/routes.rs:10`, `src/lib.rs:73,113,132,139,152`, `src/order/models.rs:143`, `src/order/repository.rs:333,336`, `src/order/routes.rs:27`, plus call sites in `src/order/repository.rs:48,101,171`.

T34 introduced wide function signatures and long expressions that exceeded rustfmt's 100-column default. The committed code was not rustfmt-clean.

**Fix**: `cargo fmt` and commit the result.

**Status**: Applied — `cargo fmt --check` exits 0; source is rustfmt-clean.

---

#### MEDIUM-2 (C-5 / BUG-5): Test isolation is fragile — observed flake in `test_admin_update_variant_sku_uniqueness` under full-suite run

**Severity**: MEDIUM (CI reliability)
**Files**: `tests/common/mod.rs:57-114` (`clean_all_tables`), `tests/product_test.rs:803`, `Makefile:30-31`

T35 ran the full PG test suite twice. First run: **1 failure** (`test_admin_update_variant_sku_uniqueness` returned 200 instead of expected 422 duplicate_error). Second run: **260/260 pass**. Re-running just that test alone or just `product_test` alone: passes.

Root cause: Cargo runs test **binaries in parallel by default** (1 OS process per `tests/*.rs` file). `--test-threads=1` only serializes tests within each binary. All binaries share the same Postgres DB at `postgres://...:5432/toko_test`. Each test calls `clean_all_tables()` in its setup, which `DELETE FROM products` mid-test in another binary, removing variant `TS-S` between this test's setup and its assertion. The dev-dep `serial_test` is declared in `Cargo.toml` but **not used** at the integration-test layer.

**Impact**: CI may report transient failures. T34 audit's "249 PASS" and T33's "238 PASS" claims happened to also depend on system load not triggering the race.

**Fix**: Add `--jobs 1` to the `Makefile` `test-pg` target to disable cargo's binary-level parallelism. Documented in README.

**Status**: Applied — `Makefile:31` uses `cargo test --jobs 1 -- --test-threads=1` for the `test-pg` target.

---

#### MEDIUM-3 (D-36 → K-14): `uq_customers_phone` is a toko-rs-only unique constraint not present in Medusa

**Severity**: MEDIUM (behavioural divergence — toko-rs rejects requests Medusa accepts)
**Files**: `migrations/007_customers_phone_unique.sql`, `migrations/sqlite/007_customers_phone_unique.sql`, `src/customer/repository.rs`
**Medusa**: `vendor/medusa/packages/modules/customer/src/models/customer.ts:1-37` — defines exactly **one** unique index: `["email", "has_account"] WHERE deleted_at IS NULL`. Phone is `model.text().searchable().nullable()` — no uniqueness.

Migration `007` adds `CREATE UNIQUE INDEX uq_customers_phone ON customers (phone) WHERE deleted_at IS NULL AND phone IS NOT NULL`. The original audit recommended dropping this index (migration 008) to match Medusa. After review, the owner decided to **keep** the constraint as a permanent, intentional divergence required by the sapa-rs integration (SMS-keyed customer lookup needs phone to be a unique identifier).

**Status**: K-14 (intentional divergence) — migration 007 is retained, migration 008 was deleted, and the divergence is documented in `docs/p1_additions.md` §3 "Customer Phone Uniqueness (K-14)". The HTTP wire shape on duplicate phone is the standard `duplicate_error` (HTTP 422) per Medusa's error contract.

**Open spec gap** (Mod 1c / Mod 1d): the Rust-side error mapping in `src/customer/repository.rs::create` (lines 38-48) and `::update` (lines 202-212) does **not** discriminate the constraint name. A duplicate-phone INSERT or UPDATE currently surfaces as `DuplicateError("Customer with email '…' already exists")` — wrong message body, correct status code. See "Spec Alignment" section below.

---

#### MEDIUM-4 (V-13): `ListOrdersParams.id` / `.status` accept only single string, not array — Medusa accepts both

**Severity**: MEDIUM
**Files**: `src/order/types.rs:64-78` (`ListOrdersParams`), `src/order/types.rs:80-94` (`AdminListOrdersParams`)
**Medusa**: `vendor/medusa/packages/medusa/src/api/store/orders/validators.ts:8-11`:

```typescript
export const StoreGetOrdersParamsFields = z.object({
  id: z.union([z.string(), z.array(z.string())]).optional(),
  status: z.union([z.string(), z.array(z.string())]).optional(),
})
```

Admin form (`packages/medusa/src/api/admin/orders/validators.ts:55-66`) is even broader: `z.union([z.string(), z.array(z.string()), createOperatorMap()])`.

toko-rs accepts only `Option<String>` for both `id`/`status` (store side) and `customer_id`/`status` (admin side). A Medusa SDK client passing `?id=ord_1&id=ord_2` or `?status[]=pending&status[]=completed` will only see the last value bound (axum query default).

**Impact**: Medium — clients filtering by multiple ids/statuses get truncated filters. Tests pass because toko-rs's own tests only send single values.

**Fix**: Change `id: Option<String>` to `id: Option<Vec<String>>` (or a custom `OneOrMany<String>` enum that deserialises both forms via serde). Update SQL builder in `list_by_customer` / `list_all` to expand into `id IN ($n, $n+1, ...)`. This change also covers `AdminListOrdersParams.customer_id` / `.status`, addressing NC-1 raised against `specs/store-modification.md` Mod 3.

**Status**: Design decision pending — design decision required on `OneOrMany<String>` serde adapter shape; not needed for current single-value P1 usage. Tracked as Deferred (P2).

---

#### MEDIUM-5 (V-14): Admin order list default limit is 50 — Medusa uses 15

**Severity**: MEDIUM (minor UX, not a compatibility break)
**File**: `src/order/types.rs:86` (`AdminListOrdersParams.limit` defaults to `types::default_limit()` = 50)
**Medusa**: `vendor/medusa/packages/medusa/src/api/admin/orders/validators.ts:49-52` — `createFindParams({ limit: 15, offset: 0 })`.

Medusa returns 15 admin orders per page by default. toko-rs returns 50. Client code that doesn't paginate but expects "the first N" gets a different N.

**Impact**: Cosmetic — affects only clients that depend on the implicit page size. Master checklist's L-7 changed default to 50 (matching Medusa **store** default) but did not differentiate admin-side defaults. Note Medusa's **admin product list** also uses 50, so the divergence is order-specific.

**Fix**: Either accept the divergence (document as K-15) or add `fn admin_default_limit() -> i64 { 15 }` and apply to `AdminListOrdersParams.limit`. This decision also resolves NC-2 raised against `specs/store-modification.md` Mod 3 (which currently encodes the 50 default).

**Status**: Design decision pending — accept K- divergence or add `admin_default_limit()`. Tracked as Deferred (P2).

---

#### MEDIUM-6 (L-18): `InvoiceConfig.is_configured()` accepts a partial config — invoice generation succeeds with blank fields

**Severity**: MEDIUM (functional edge case)
**File**: `src/config.rs:56-63`, `src/invoice/routes.rs:46-48`

Original code used `||` (any). If admin set only `INVOICE_COMPANY_NAME=Foo` and left address/phone/email blank, `is_configured()` returned true and `GET /admin/orders/:id/invoice` succeeded — but emitted an invoice with blank issuer address/phone/email.

**Fix**: Change `||` to `&&` for the 4 core required fields (`company_name`, `company_address`, `company_phone`, `company_email`).

**Status**: Applied — verified at `src/config.rs:57-62`: `&&` combines all four required fields.

---

### LOW / Documentation

#### LOW-1 (X-15): `OrderSummary` has no `raw_*` mirror fields — Medusa returns 8 raw fields alongside the 8 typed ones

**Severity**: LOW (P2-shape — BigNumber semantics not applicable to i64 storage)
**File**: `src/order/models.rs:5-15`
**Medusa**: `OrderSummaryDTO` includes `raw_pending_difference`, `raw_current_order_total`, … 8 raw mirrors of the typed fields, each typed `BigNumberRawValue`.

Medusa's `BigNumber` representation includes both a `value` (computed) and a `raw` (precision-preserving) form. toko-rs uses `i64` (cents) throughout, which has no `raw_*` analog.

**Impact**: Clients reading `order.summary.raw_paid_total` get undefined. Most clients won't read raw values.

**Fix**: Document as P2 deferred (X-15: BigNumber/raw representation). No P1 action needed.

**Status**: Deferred (P2) — BigNumber/raw representation has no analog in fixed-precision storage; not actionable until a precision-sensitive currency module is introduced.

---

#### LOW-2: `payment_records.metadata` is read from DB but the write path silently ignores it

**Severity**: LOW (no current write path passes metadata, but the gap is invisible)
**File**: `src/payment/models.rs:13-14`, `src/payment/repository.rs:24-37,47-60`

The PaymentRecord model has `#[serde(skip_deserializing)] pub metadata: ...`. But `create()` and `create_with_tx()` INSERT statements don't bind `metadata` — they only set `id, order_id, amount, currency_code, status='pending', provider='manual'`. No method ever writes payment metadata.

**Impact**: Future webhook integrations or provider-specific metadata cannot be stored without code changes. Master checklist's D-20 added `deleted_at` to payment_records but never wired metadata.

**Fix**: Add `metadata: Option<HashMap<String, Value>>` to a future payment input type, bind in INSERT. Optional for P1.

**Status**: Deferred (P2) — non-blocking; needed only when payment provider integration arrives.

---

#### LOW-3: `OrderResponse` does not skip-serialize order's `metadata` when empty (false alarm)

**Severity**: LOW (cosmetic — null vs missing)
**File**: `src/order/models.rs:32`

When no metadata is set, the response includes `"metadata": null`. Medusa serializes `metadata: null` too for consistency. **This is actually correct** — false alarm. Documented to prove the check ran.

**Status**: Applied (no change required — current behaviour matches Medusa).

---

#### LOW-4 (K-13): `Order.cart_id` is serialized in all responses — Medusa hides it

**Severity**: LOW (info-leak: internal cart→order link)
**File**: `src/order/models.rs:20-22` (`pub cart_id: Option<String>`)
**Medusa**: `vendor/medusa/packages/medusa/src/api/store/orders/query-config.ts:16-65` — no `cart_id` in `defaultStoreOrderFields` or `defaultStoreRetrieveOrderFields`.

toko-rs originally returned `cart_id` in every order response. Adding `#[serde(skip)]` hides it from JSON while leaving the field available for sqlx deserialization and cross-module queries.

**Fix**: Add `#[serde(skip)]` to `Order.cart_id`.

**Status**: Applied — verified at `src/order/models.rs:21-22`: `#[serde(skip)] pub cart_id: Option<String>`.

---

#### LOW-5 (C-7): Test cleanup helper does not delete `product_images`

**Severity**: LOW (no test failures observed; FK CASCADE saves it, but it's still a gap)
**File**: `tests/common/mod.rs:57-114`

The PG/SQLite FK on `product_images.product_id` is `ON DELETE CASCADE`, so `DELETE FROM products` cascades. But future direct inserts into `product_images` (e.g., for fixture data) would not be cleaned.

**Fix**: Add `sqlx::query("DELETE FROM product_images").execute(pool).await.unwrap();` to `clean_all_tables`.

**Status**: Applied — verified at `tests/common/mod.rs:102-105`: `DELETE FROM product_images` runs between `product_variants` and `products`.

---

#### LOW-6: README "tests/" tree omits `e2e/` enumeration

**Severity**: LOW (doc-only)
**File**: `README.md:209-219`

README's `tests/` tree shows `e2e/` as a directory. The actual structure has `tests/e2e/main.rs` plus `tests/e2e/common/mod.rs` plus individual spec files. Cosmetic, not a real bug.

**Status**: Applied (no change required — accepted as documentation level of detail).

---

### BUG (Functional)

#### BUG-1 (B-35 part 1): `admin_capture_payment` and friends don't filter `deleted_at IS NULL` on payment_records

**Severity**: BUG (latent — depends on payment soft-delete adoption)
**File**: `src/payment/repository.rs` (all 3 methods)

Originally, `capture_by_order_id`, `cancel_by_order_id`, and `find_by_order_id` did not filter `AND deleted_at IS NULL` despite D-20 having added the column. A soft-deleted payment could be undeleted-by-side-effect via these UPDATEs, and the order's payment status resolver could report a soft-deleted payment.

**Fix**: Add `AND deleted_at IS NULL` to all three queries.

**Status**: Applied — verified at `src/payment/repository.rs:68` (`find_by_order_id`), `78` (`cancel_by_order_id`), and `89` (`capture_by_order_id`); all three filter `AND deleted_at IS NULL`.

---

#### BUG-2 (B-34): `admin_cancel_order` ignores the payment-cancel error — silent failure

**Severity**: BUG (silent failure)
**File**: `src/order/routes.rs:87-95`

Originally, `admin_cancel_order` used `let _ = state.repos.payment.cancel_by_order_id(&id).await;` which discarded the result. If the DB call failed (connection drop, serialization failure), the order would be canceled but the payment cancel would silently not happen.

**Fix**: Propagate the error from `cancel_by_order_id` instead of discarding it. A future P2 task can wrap both operations in a transaction for atomic semantics.

**Status**: Applied (error-propagation half) — verified at `src/order/routes.rs:93`: `state.repos.payment.cancel_by_order_id(&id).await?;` uses `?`. Transaction-wrapping half deferred to P2 (invasive refactor).

---

#### BUG-3 (B-35 part 2): `resolve_payment_status` doesn't filter `deleted_at IS NULL`

**Severity**: BUG (latent — depends on payment soft-delete adoption)
**File**: `src/order/repository.rs:344-361`

The order's payment-status resolver originally read `SELECT status, amount FROM payment_records WHERE order_id = $1` without filtering soft-deleted rows. The order response could show `payment_status: captured` for a soft-deleted payment.

**Fix**: Append `AND deleted_at IS NULL`.

**Status**: Applied — verified at `src/order/repository.rs:346`: query includes `AND deleted_at IS NULL`.

---

#### BUG-4 (B-33): Order state-transition methods use a TOCTOU pattern — read-then-update with no guard predicate

**Severity**: BUG (concurrency hazard)
**File**: `src/order/repository.rs:363-441` — methods `cancel_order`, `complete_order`, `fulfill_order`, `ship_order`.

Originally the pattern was: SELECT → guard check → unconditional UPDATE. Two concurrent admins racing on `POST /admin/orders/X/fulfill` would both SELECT a `not_fulfilled` row, both pass the guard, and both UPDATE — silently double-fulfilling without idempotency, or overwriting `shipped_at`.

**Fix**: Include the guard predicate in the UPDATE's WHERE clause and check `rows_affected() == 0` → return 400 with the appropriate message. Apply to all 4 transition methods.

**Status**: Applied — verified at `src/order/repository.rs`:
- `cancel_order` line 367-368: `WHERE id = $1 AND status != 'canceled' AND status != 'completed'` + `rows_affected() == 0` check (line 374).
- `complete_order` line 387-388: `WHERE id = $1 AND status != 'completed' AND status != 'canceled'` + check (line 394).
- `fulfill_order` line 407-408: `WHERE id = $1 AND fulfillment_status = 'not_fulfilled' AND status != 'canceled'` + check (line 414).
- `ship_order` line 427-428: `WHERE id = $1 AND fulfillment_status = 'fulfilled' AND status != 'canceled'` + check (line 434).

---

#### BUG-5: `update_variant_sku_uniqueness` test flaked under full-suite parallel execution

**Severity**: BUG (test infrastructure)
**File**: `tests/product_test.rs:803`, `tests/common/mod.rs:57`

Already detailed in MEDIUM-2 (C-5). Recorded under BUG to surface the real reliability concern: CI may show transient 422 failures whenever cargo runs binary-level test parallelism.

**Status**: Applied — see C-5.

---

## Already correct (verified)

| Area | Status |
|------|--------|
| Error mapping (`AppError` → 8 status codes) | PASS — matches Medusa OAS error enum |
| Cart completion idempotency (`cart_id UNIQUE` + `FOR UPDATE`) | PASS — exhaustive idempotency check before INSERT |
| Cart line item delete response `{id, object: "line-item", deleted, parent}` | PASS — matches `vendor/medusa/.../carts/[id]/line-items/[line_id]/route.ts:58` |
| Product delete response `{id, object: "product", deleted}` | PASS — matches Medusa |
| Variant delete response `{id, object: "variant", deleted, parent}` | PASS — matches Medusa |
| Product option delete response `{id, object: "product_option", deleted, parent}` | PASS — matches Medusa `[option_id]/route.ts:80` |
| `email: Option<String>` on customer create with `validate(email)` | PASS — Medusa's `nullish().email()` matches |
| `quantity: gte(0)` on cart line update with `0 → delete` branch | PASS (B-27) |
| `bool_or_string` deserialization on `is_giftcard`/`discountable` | PASS (B-29) |
| SQL injection guard on ORDER BY (whitelist 15 columns) | PASS (B-18) |
| Order ownership check on `GET /store/orders/{id}` | PASS (B-16) |
| Snapshot `variant_option_values` via 3-way JOIN | PASS (B-3) |
| `metadata` and addresses preserved cart→order | PASS (B-19) |
| `OrderSummary.paid_total` computed from captured payments | PASS (L-17) |
| Fulfill/ship state machine guards (cancel-guard, double-fulfill, fulfill-before-ship) | PASS (L-15) — including the TOCTOU fix now in BUG-4 |

---

## Spec Alignment (`specs/store-modification.md`)

This section reconciles the audit findings with the 4 sapa-rs integration modifications. Per the spec, K-14 (formerly D-36) is an intentional, permanent divergence.

### Mod 1 — Phone unique constraint

**Mod 1a — PG migration `007_customers_phone_unique.sql`**: **Applied**. File present at `migrations/007_customers_phone_unique.sql`.

**Mod 1b — SQLite migration `007_customers_phone_unique.sql`**: **Applied**. File present at `migrations/sqlite/007_customers_phone_unique.sql`. Migrations directory listing confirms **7 PG + 7 SQLite** files (`001` … `007`), no `008` (the abandoned drop-migration is gone).

**Mod 1c — `create` constraint-name discrimination**: **Open gap**. `src/customer/repository.rs:38-48` recognises the generic unique-violation code but does **not** branch on `db_err.constraint() == Some("uq_customers_phone")`. A duplicate-phone INSERT produces `DuplicateError("Customer with email '…' already exists")` — the HTTP 422 is correct but the human-readable message names the wrong field. Implement per spec §1c.

**Mod 1d — `update` constraint-name discrimination**: **Open gap**. `src/customer/repository.rs:200-212` has the same shape as `create` and the same gap. Implement per spec §1d.

**K-14 documentation**: **Applied**. `docs/p1_additions.md` §3 carries a "Customer Phone Uniqueness (K-14)" subsection documenting the intentional 422 behaviour and the sapa-rs rationale.

### Mod 2 — Phone filter on `GET /admin/customers`

**Status**: **Fully applied.**

- `src/customer/types.rs:49` declares `pub phone: Option<String>`.
- `src/customer/repository.rs:77` captures `phone_filter`.
- WHERE-builder at `src/customer/repository.rs:101-104` adds `c.phone = ${param_idx}` (exact match, not ILIKE — per spec).
- Count-query binding at `src/customer/repository.rs:128-130` and data-query binding at `153-155` both bind `phone_filter` before `has_account_val`.

### Mod 3 — `GET /admin/orders` (list all orders)

**Status**: **Fully applied.**

- `AdminListOrdersParams` defined at `src/order/types.rs:80-94` with `customer_id`, `status`, `offset`, `limit` and a `capped_limit()` helper.
- `list_all` defined at `src/order/repository.rs:263-324`: builds dynamic WHERE with optional `customer_id` / `status` filters, ORDER BY `created_at DESC`, count + data queries, calls `load_items` per row.
- Route registered at `src/order/routes.rs:24`: `.route("/admin/orders", get(admin_list_orders))`.
- Handler at `src/order/routes.rs:106-119`.

**Residual concerns**:
- NC-1 (V-13): Mod 3 codifies `Option<String>` for `customer_id`/`status`, conflicting with Medusa's union shape — Deferred (P2).
- NC-2 (V-14): Mod 3 uses `default_limit() = 50` for admin, while Medusa admin uses 15 — Deferred (P2).
- NC-5: Mod 3 has no `id` multi-value filter — covered by NC-1 fix.
- NC-6 (I-1): `list_all` inherits the N+1 `load_items` pattern; acceptable for P1.

### Mod 4 — `GET /admin/orders/{id}` (admin detail)

**Status**: **Fully applied.**

- Route registered at `src/order/routes.rs:25`.
- Handler at `src/order/routes.rs:121-128`: no ownership check (correctly omitted for admin), reuses `find_by_id`.

### Spec gaps still open

| Gap | Action | Priority |
|---|---|---|
| **Mod 1c** | `customer/repository.rs::create` — branch on `db_err.constraint() == Some("uq_customers_phone")` and emit phone-specific message | Trivial, do first |
| **Mod 1d** | `customer/repository.rs::update` — same branch on UPDATE path | Trivial, do alongside 1c |
| Test coverage | Integration tests for duplicate-phone POST + UPDATE asserting 422 + correct message body | Should accompany Mod 1c/1d |

The HTTP status (422 via `AppError::DuplicateError`) is already correct in both methods; only the message body needs the discrimination.

---

## Summary table

| ID | Severity | Category | Finding | File | Status |
|---|---|---|---|---|---|
| S-38 | HIGH | Response shape | `OrderSummary` missing `credit_line_total` field (8th Medusa-required field) | `src/order/models.rs:5-15` | Applied |
| S-39 | HIGH | Response shape | `CartWithItems` missing `item_discount_total` field | `src/cart/models.rs:92-126` | Applied |
| Doc-1 | HIGH | Doc drift | `docs/p1_additions.md` §2 misclassifies `complete` as addition; Medusa has the endpoint | `docs/p1_additions.md` | Applied |
| Doc-2 | LOW | Doc drift | `README.md:271` said "259 tests"; actual 260 | `README.md:271` | Applied |
| X-15 | LOW | Response shape | `OrderSummary` missing 8 `raw_*` BigNumber mirror fields | `src/order/models.rs:5-15` | Deferred (P2) |
| K-13 | LOW | Response shape | `Order.cart_id` exposed; Medusa's `defaultStoreRetrieveOrderFields` omits it | `src/order/models.rs:20-22` | Applied |
| V-13 | MEDIUM | Input/validation | `ListOrdersParams.id`/`.status` accept single string only; Medusa accepts arrays | `src/order/types.rs:64-94` | Design decision pending |
| V-14 | MEDIUM | Input/validation | Admin order list `limit` defaults to 50; Medusa admin defaults to 15 | `src/order/types.rs:86` | Design decision pending |
| D-36 | MEDIUM | DB schema | `uq_customers_phone` unique index has no Medusa counterpart | `migrations/007_customers_phone_unique.sql` | K-14 (intentional divergence) |
| B-33 | BUG | Concurrency | Order state transitions use read-then-update with no UPDATE guard | `src/order/repository.rs:363-441` | Applied |
| B-34 | BUG | Silent failure | `admin_cancel_order` discarded payment-cancel error via `let _ = …` | `src/order/routes.rs:87-95` | Applied (error-propagation half; tx-wrapping deferred to P2) |
| B-35 | BUG | Soft-delete leakage | Payment repository + resolver queries missing `deleted_at IS NULL` | `src/payment/repository.rs:63-97`, `src/order/repository.rs:344-361` | Applied |
| L-18 | MEDIUM | Business logic | `InvoiceConfig.is_configured()` used OR over 4 fields | `src/config.rs:56-63` | Applied |
| C-5 | MEDIUM | Test infrastructure | Test isolation racy — multiple binaries run in parallel against shared DB | `tests/common/mod.rs:57`, `Makefile:31` | Applied |
| C-6 | MEDIUM | Quality gate | `cargo fmt --check` failed with 703 lines of drift after T34 | multiple | Applied |
| C-7 | LOW | Test infrastructure | `clean_all_tables` omitted `product_images` | `tests/common/mod.rs:102-105` | Applied |

### Spec modifications

| Mod | Description | Status |
|---|---|---|
| Mod 1a | PG migration `007_customers_phone_unique.sql` | Applied |
| Mod 1b | SQLite migration `007_customers_phone_unique.sql` | Applied |
| Mod 1c | `create` constraint-name discrimination | **To implement** |
| Mod 1d | `update` constraint-name discrimination | **To implement** |
| Mod 2 | `phone` filter on `AdminCustomerListParams` | Applied |
| Mod 3 | `GET /admin/orders` + `AdminListOrdersParams` + `list_all` | Applied |
| Mod 4 | `GET /admin/orders/{id}` admin detail | Applied |

---

## Counts by status

| Bucket | Count | IDs |
|---|---|---|
| **Applied** (audit findings already in source) | 11 | S-38, S-39, Doc-1, Doc-2, K-13, B-33, B-34 (propagation half), B-35, L-18, C-5, C-6, C-7 |
| **Deferred (P2)** | 1 | X-15 |
| **Design decision pending** | 2 | V-13, V-14 |
| **K- (intentional divergence)** | 1 | K-14 (formerly D-36) |
| **Spec gaps to implement** | 2 | Mod 1c, Mod 1d |

Total = 15 audit findings + 1 K-14 reclassification + 2 spec-mod gaps = 18 tracked items, of which **11 applied, 3 deferred or pending design decision, 1 documented divergence, 2 concrete spec gaps**.

---

## Recommended action plan

### Group A — Spec gaps (trivial, do first)

1. **Mod 1c** — `src/customer/repository.rs::create` — wrap the unique-violation branch with a `db_err.constraint() == Some("uq_customers_phone")` check that returns `DuplicateError("Customer with phone '…' already exists")`. Falls through to the existing email message otherwise.
2. **Mod 1d** — `src/customer/repository.rs::update` — same constraint discrimination.
3. **Test coverage** — integration tests POSTing two customers with the same `phone` (assert 422 + correct message) and the corresponding UPDATE path. Bump README test count after merging.

### Group B — Design decisions (no code action without owner sign-off)

| ID | Decision needed |
|---|---|
| V-13 / NC-1 | Multi-value `id`/`status` filter shape — `OneOrMany<String>` serde shim, or accept divergence as K- |
| V-14 / NC-2 | Admin order list default limit — Medusa parity (15) or status quo (50) |

### Group C — Documentation maintenance

4. `docs/p1_additions.md` §3 K-14 subsection — already present (no change).
5. `docs/audit-master-checklist.md` — K-14 row should reference Mod 1c/1d as the remaining concrete work.
6. `README.md` — test count is current (260); bump if Group A adds new tests.

---

## Bottom line

- **All 15 audit findings are tractable**: 11 applied on disk, 1 deferred (X-15), 2 awaiting design decision (V-13, V-14), 1 reclassified as K-14.
- **0 BUG- or HIGH-severity audit items remain open.**
- **The only remaining concrete code work** for this spec is **Mod 1c + Mod 1d** — distinguishing `uq_customers_phone` from `uq_customers_email_has_account` in `create`/`update` error mapping. HTTP status (422) is already correct; only the message body is misleading.
