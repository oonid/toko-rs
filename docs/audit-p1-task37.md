# Task 37: Audit — P1 Webhook Extension: Transactional Event Outbox + PostgreSQL LISTEN/NOTIFY

**Date**: 2026-05-25
**Medusa vendor**: `40a60e85b1` (v2.15.2 / develop, unchanged from T35)
**Scope**: Scope and design audit for extending P1 MVP with a durable event notification mechanism. No Medusa v2 equivalent exists — this is a deliberate toko-rs addition. No code has been written yet; this document defines what to build and why.
**Status**: 0 lines implemented. All items tracked as W-1…W-8 (to implement).

---

## Methodology

| Source | Purpose |
|--------|---------|
| `vendor/medusa/packages/modules/event-bus-redis/src/services/event-bus-redis.ts` | Medusa in-process event bus (Redis/BullMQ) — reference for event naming convention |
| `vendor/medusa/packages/medusa/src/api/hooks/payment/[provider]/route.ts` | Medusa's only "webhook" — inbound from payment providers, not outbound |
| `vendor/medusa/packages/core/utils/src/core-flows/events.ts` | Medusa event name constants (`order.placed`, `order.canceled`, etc.) |
| `src/order/routes.rs`, `src/cart/routes.rs` | 5 mutation handlers that will emit events |
| `src/payment/repository.rs`, `src/order/repository.rs` | Repositories needing `_tx` variants |
| `migrations/001–007_*.sql` | Migration numbering — next is 008 |
| `docs/p1_additions.md §4` | "Webhooks … No event system in P1" — entry to update |

---

## 1. Medusa Context

**No outbound webhook primitive exists in Medusa v2 core.** Findings confirmed in vendor exploration:

- `IEventBusModuleService` — in-process pub/sub backed by Redis (BullMQ) or in-memory. Subscriber functions run in the same Node.js process. External consumers are not addressable.
- The only webhook-shaped code is **inbound**: `POST /hooks/payment/:provider` receives Stripe/PayPal events. Medusa does not POST to external URLs.
- No `webhook_subscriptions` table in any module migration. No admin endpoint for webhook management.
- Community plugins add outbound webhooks as a custom subscriber that does `fetch(url)` — not part of core.

This makes the webhook mechanism a **toko-rs extension**, classified alongside K-11 (admin cart listing) and K-12 (invoice system). It extends P1 without breaking Medusa API compatibility.

---

## 2. Chosen Approach: Transactional Outbox + LISTEN/NOTIFY

### Why not LISTEN/NOTIFY alone

PG NOTIFY messages are ephemeral. If laku-rs is restarting when `order.placed` fires, the notification is silently dropped and the order event is permanently lost. That's a silent data loss scenario unacceptable for order-driven integrations.

### Why not outbox-only polling

Polling with no push signal works but imposes latency equal to the poll interval and creates steady DB load even when nothing is happening.

### Combined approach (chosen)

Two components work together:

**Outbox (durability)**  
Each order mutation writes a row to `event_outbox` inside the **same DB transaction** as the mutation. If the mutation rolls back, the event rolls back too. If the app crashes before delivering a notification, the row is still there to be read on the next poll. Events are never lost.

**LISTEN/NOTIFY (latency)**  
After the transaction commits, a non-transactional `SELECT pg_notify('toko_events', event_id)` is issued. This wakes up any active Postgres `LISTEN` subscriber (e.g. laku-rs) in near-real-time. If the notify fails (consumer down, network blip), the outbox row still exists — the consumer finds it on the next poll. The notify is purely a hint; it carries only the event ID, not the payload.

**SQLite compatibility**  
`pg_notify` is Postgres-only. All NOTIFY calls are wrapped in `#[cfg(feature = "postgres")]`. SQLite builds skip the notify; the outbox write still happens, giving parity for test isolation.

---

## 3. New Database Schema — Migration 008

### `migrations/008_event_outbox.sql` (PostgreSQL)

```sql
CREATE TABLE event_outbox (
    id            TEXT PRIMARY KEY,
    event_name    TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id   TEXT NOT NULL,
    payload       JSONB NOT NULL DEFAULT '{}',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at  TIMESTAMPTZ
);

-- Fast unprocessed scan (only unprocessed rows indexed)
CREATE INDEX idx_event_outbox_unprocessed
    ON event_outbox (created_at ASC)
    WHERE processed_at IS NULL;

-- Lookup all events for a given resource
CREATE INDEX idx_event_outbox_resource
    ON event_outbox (resource_type, resource_id);
```

### `migrations/sqlite/008_event_outbox.sql` (SQLite)

```sql
CREATE TABLE event_outbox (
    id            TEXT PRIMARY KEY,
    event_name    TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id   TEXT NOT NULL,
    payload       TEXT NOT NULL DEFAULT '{}',
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    processed_at  TEXT
);
CREATE INDEX idx_event_outbox_unprocessed
    ON event_outbox (created_at ASC)
    WHERE processed_at IS NULL;
CREATE INDEX idx_event_outbox_resource
    ON event_outbox (resource_type, resource_id);
```

**Schema rationale**:

| Column | Decision |
|--------|---------|
| `event_name` | Follows Medusa convention: `order.placed`, `order.canceled`, `order.fulfillment_created`, `order.shipment_created`, `payment.captured` |
| `resource_type` + `resource_id` | Lets consumers re-fetch the live resource by ID without parsing payload |
| `payload JSONB` | Snapshot of the affected resource at event time — avoids an extra GET for most consumer use cases |
| `processed_at` nullable | Soft-mark for consumer ACK. `NULL` = unprocessed. No hard delete — rows are an audit log; GC is an operator concern (P2) |
| Partial index on `processed_at IS NULL` | `GET /admin/events?unprocessed_only=true` is O(unprocessed) not O(table) |

---

## 4. New Module: `src/event/`

```
src/event/
├── mod.rs        — pub use; wire into Repositories; register routes
├── models.rs     — EventOutboxRow (sqlx::FromRow, serde::Serialize)
├── repository.rs — EventRepository (insert_event, notify_event, find_unprocessed, mark_processed, list_all)
├── routes.rs     — admin_list_events, admin_mark_event_processed
└── types.rs      — EventListParams, EventResponse, EventListResponse
```

### Key signatures

**`insert_event`** — called by order/cart handlers; must share the mutation transaction:

```rust
pub async fn insert_event(
    &self,
    tx: &mut Transaction<'_, Postgres>,   // same tx as the mutation
    event_name: &str,
    resource_type: &str,
    resource_id: &str,
    payload: serde_json::Value,
) -> Result<EventOutboxRow, AppError>
```

SQLite variant uses `Transaction<'_, Sqlite>`. Can be unified via a trait bound or kept as feature-gated methods matching the pattern elsewhere in the codebase.

**`notify_event`** — called after `tx.commit()`; outside the transaction:

```rust
#[cfg(feature = "postgres")]
pub async fn notify_event(
    pool: &PgPool,
    event_id: &str,
) -> Result<(), AppError> {
    sqlx::query("SELECT pg_notify('toko_events', $1)")
        .bind(event_id)
        .execute(pool)
        .await?;
    Ok(())
}
```

A crash between `commit()` and `notify_event()` is tolerable — the outbox row persists; the consumer will find it on the next poll.

---

## 5. Emission Points

Five mutation handlers gain outbox + notify wiring:

| Handler | File | Event name | resource_type | resource_id |
|---------|------|-----------|---------------|-------------|
| `store_complete_cart` (cart→order) | `src/cart/routes.rs` | `order.placed` | `order` | new `order.id` |
| `admin_cancel_order` | `src/order/routes.rs` | `order.canceled` | `order` | `order.id` |
| `admin_fulfill_order` | `src/order/routes.rs` | `order.fulfillment_created` | `order` | `order.id` |
| `admin_ship_order` | `src/order/routes.rs` | `order.shipment_created` | `order` | `order.id` |
| `admin_capture_payment` | `src/order/routes.rs` | `payment.captured` | `order` | `order.id` |

**Handler refactor pattern** (same for all five):

```rust
// Before (direct pool call, no transaction)
async fn admin_cancel_order(State(state): State<AppState>, Path(id): Path<String>) -> ... {
    let order = state.repos.order.cancel_order(&id).await?;
    state.repos.payment.cancel_by_order_id(&id).await?;
    Ok(Json(OrderResponse { order }))
}

// After (shared transaction + outbox insert + notify)
async fn admin_cancel_order(State(state): State<AppState>, Path(id): Path<String>) -> ... {
    let mut tx = state.pool.begin().await?;
    let order = state.repos.order.cancel_order_tx(&id, &mut tx).await?;
    state.repos.payment.cancel_by_order_id_tx(&id, &mut tx).await?;
    state.repos.event.insert_event(
        &mut tx, "order.canceled", "order", &id,
        serde_json::to_value(&order).unwrap_or_default(),
    ).await?;
    tx.commit().await?;
    #[cfg(feature = "postgres")]
    let _ = state.repos.event.notify_event(&state.pool, &order.id).await;
    Ok(Json(OrderResponse { order }))
}
```

This also completes the **B-34 deferred item** from T35: `admin_cancel_order` now wraps order cancel + payment cancel in a single transaction (the P2 refactor that was deferred is resolved here as part of the `_tx` variant work).

### Repository `_tx` variants required

| Repository | New method | Notes |
|------------|-----------|-------|
| `src/order/repository.rs` | `cancel_order_tx` | Accepts `&mut Transaction` |
| `src/order/repository.rs` | `complete_order_tx` | Accepts `&mut Transaction` |
| `src/order/repository.rs` | `fulfill_order_tx` | Accepts `&mut Transaction` |
| `src/order/repository.rs` | `ship_order_tx` | Accepts `&mut Transaction` |
| `src/payment/repository.rs` | `cancel_by_order_id_tx` | Accepts `&mut Transaction` |
| `src/payment/repository.rs` | `capture_by_order_id_tx` | Accepts `&mut Transaction` |

Existing non-`_tx` methods are **kept unchanged** — they are used by tests and by `GET` handlers that don't modify state.

---

## 6. Admin API Endpoints

Two new admin routes expose the outbox to laku-rs and operators. laku-rs can choose polling-only (via HTTP) or LISTEN-driven; both paths are supported.

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/admin/events` | List outbox events. Query params: `limit`, `offset`, `after` (ISO timestamp cursor), `resource_type`, `unprocessed_only` (bool) |
| `POST` | `/admin/events/:id/mark-processed` | Set `processed_at = now()`. Consumer ACK. Returns `{ event }` |

**`EventListParams`**:
```rust
pub struct EventListParams {
    pub limit: Option<i64>,           // default 50, max 100
    pub offset: Option<i64>,          // default 0
    pub after: Option<DateTime<Utc>>, // cursor: created_at > after
    pub resource_type: Option<String>,
    pub unprocessed_only: Option<bool>,
}
```

**Response shape** for a single event:
```json
{
  "id": "01j...",
  "event_name": "order.placed",
  "resource_type": "order",
  "resource_id": "ord_01j...",
  "payload": { "id": "ord_01j...", "status": "pending", ... },
  "created_at": "2026-05-25T10:00:00Z",
  "processed_at": null
}
```

---

## 7. `docs/p1_additions.md` Updates Required

### Section 2 — toko-rs Additions

Add new sub-table:

```markdown
### Admin: Event Outbox
| Method | Path | Notes |
|--------|------|-------|
| GET  | `/admin/events`                     | List event outbox. Supports `after` cursor, `resource_type`, `unprocessed_only` filters. |
| POST | `/admin/events/:id/mark-processed`  | Mark event as processed (consumer ACK). |
```

### Section 4 — Medusa v2 Features Deferred from P1

Replace the "Webhooks" row:

| Before | After |
|--------|-------|
| `Webhooks \| /admin/api-keys, event bus \| No event system in P1` | Split into two rows: (1) `Outbound HTTP webhooks \| /admin/webhooks \| HTTP dispatcher + retry queue deferred to P2`; (2) row removed — event outbox is now in section 2. |

---

## 8. Scope Estimate

| Component | Effort |
|-----------|--------|
| Migration 008 (PG + SQLite) + `clean_all_tables` update | 0.5 day |
| `src/event/` module (models, repo, types, routes) | 1 day |
| Repository `_tx` variants (6 methods across 2 files) | 1 day |
| Emission points wired in 5 handlers | 0.5 day |
| Admin endpoints + integration tests (9 tests) | 1 day |
| `notify_event` + LISTEN smoke test | 0.5 day |
| Documentation updates | 0.5 day |
| **Total** | **~5 days** |

---

## 9. Integration Tests Plan

New test file: `tests/event_test.rs`

| Test | What it verifies |
|------|-----------------|
| `test_order_placed_creates_event_row` | Complete cart → `event_outbox` row with `event_name = "order.placed"`, `resource_id = order.id` |
| `test_order_canceled_creates_event_row` | `POST /admin/orders/:id/cancel` → `order.canceled` row in outbox |
| `test_order_fulfilled_creates_event_row` | `POST /admin/orders/:id/fulfill` → `order.fulfillment_created` row |
| `test_order_shipped_creates_event_row` | `POST /admin/orders/:id/ship` → `order.shipment_created` row |
| `test_payment_captured_creates_event_row` | `POST /admin/orders/:id/capture-payment` → `payment.captured` row |
| `test_admin_list_events_returns_events` | `GET /admin/events` returns the 5 rows above |
| `test_admin_list_events_filter_by_resource_type` | `?resource_type=order` excludes `payment.captured` row |
| `test_admin_mark_event_processed` | `POST /admin/events/:id/mark-processed` sets `processed_at` non-null |
| `test_admin_list_events_unprocessed_only` | `?unprocessed_only=true` excludes the just-processed row |

Also update `tests/common/mod.rs::clean_all_tables()` to add `DELETE FROM event_outbox`.

---

## 10. Findings Summary

| ID | Category | Finding | Status |
|----|----------|---------|--------|
| W-1 | Schema | Migration 008: `event_outbox` table (PG + SQLite) | To implement |
| W-2 | Module | `src/event/` — models, repository, types, routes | To implement |
| W-3 | Repository | `_tx` variants for 5 order + 1 payment method | To implement (also closes B-34 deferred tx item) |
| W-4 | Emission | Outbox insert + NOTIFY wired into 5 mutation handlers | To implement |
| W-5 | API | `GET /admin/events` + `POST /admin/events/:id/mark-processed` | To implement |
| W-6 | Tests | 9 integration tests in `tests/event_test.rs` + `clean_all_tables` update | To implement |
| W-7 | Docs | Update `docs/p1_additions.md §2` + `§4`; README endpoint/test counts | To implement |
| W-8 | Infra | `clean_all_tables` must `DELETE FROM event_outbox` | To implement alongside W-1 |

---

## Bottom Line

- **Medusa has no outbound webhook primitive** — this is a confirmed net-new toko-rs addition, not a compliance gap.
- **Transactional outbox + NOTIFY** is the right architecture: the outbox guarantees zero event loss even on crash or consumer downtime; NOTIFY gives near-real-time delivery at zero extra cost.
- **laku-rs integration is simple from day one**: it can start by polling `GET /admin/events?unprocessed_only=true&after=<cursor>` over HTTP without any PG LISTEN setup, then upgrade to LISTEN-driven consumption independently.
- **B-34 deferred work is resolved here for free**: the `_tx` variants required for atomic event emission also give the order cancel + payment cancel atomicity that was deferred in T35.
- **Scope is bounded**: no new infrastructure dependencies for PG builds. SQLite builds retain full parity minus the NOTIFY signal. ~5 days to a working, tested P1 webhook foundation.
