# Task 32: P1 Admin Extensions — Customer List, Cart List, Order Cancel/Complete

**Date**: 2026-05-01
**Medusa vendor**: `0303d7f30b` (latest develop branch)
**Scope**: 5 new admin endpoints (2 customer, 1 cart, 2 order)
**Status**: All findings applied

## Methodology

1. Read Medusa vendor source for admin customer routes (`packages/medusa/src/api/admin/customers/`), validators, and query configs
2. Read toko-rs implementation: routes, types, repository for all 5 new endpoints
3. Compare request/response shapes, filter params, pagination, error handling
4. Verify test coverage for all happy paths and error paths
5. Check migrations for schema changes (phone index, canceled status)

---

## Endpoints Implemented (5)

| # | Endpoint | Method | Path | Response wrapper | Medusa Parity |
|---|----------|--------|------|-----------------|---------------|
| 1 | Admin list customers | GET | /admin/customers | `{customers, count, offset, limit}` | MATCH |
| 2 | Admin get customer | GET | /admin/customers/:id | `{customer}` | MATCH |
| 3 | Admin list carts | GET | /admin/carts | `{carts, count, offset, limit}` | EXTENSION (K-11) |
| 4 | Admin cancel order | POST | /admin/orders/:id/cancel | `{order}` | MATCH (simplified) |
| 5 | Admin complete order | POST | /admin/orders/:id/complete | `{order}` | EXTENSION |
| 6 | Admin get invoice config | GET | /admin/invoice-config | `{invoice_config}` | EXTENSION (K-12) |
| 7 | Admin update invoice config | POST | /admin/invoice-config | `{invoice_config}` | EXTENSION (K-12) |
| 8 | Admin get order invoice | GET | /admin/orders/:id/invoice | `{invoice}` | EXTENSION (K-12) |

---

## Checklist Entries Applied (9)

| ID | Finding | Fix | Section |
|----|---------|-----|---------|
| S-32 | Admin customer list+get endpoints missing | Added 2 endpoints with list filters and pagination | 32a |
| S-33 | Admin cart list endpoint missing | Added `GET /admin/carts` with `id`, `customer_id` filters (K-11) | 32b |
| S-34 | Admin order cancel/complete missing | Added 2 endpoints with simplified logic | 32c,32d |
| S-35 | Invoice config + invoice generation endpoints missing | Added 3 endpoints. `invoice_config` table, invoice from order data | 32e |
| D-29 | No index on `customers.phone` for admin search | Added partial index `WHERE deleted_at IS NULL AND phone IS NOT NULL` | 32a.8 |
| D-30 | `payment_records.status` CHECK missing `'canceled'` | Added `'canceled'` to CHECK constraint in both PG and SQLite | 32c.4 |
| D-31 | No `invoice_config` table for invoice issuer information | `CREATE TABLE invoice_config` in both PG and SQLite. Migration 007 | 32e |
| L-10 | Order cancel simplified — no payment provider calls | P1: update order status + payment status only | 32c,32d |
| L-11 | Invoice generated on-the-fly — no `invoices` table | P1: `Invoice::from_order()` merges config + order. No PDF. | 32e |

---

## Customer List — Medusa Comparison

### Filter params

| Param | Medusa | toko-rs | Status |
|-------|--------|---------|--------|
| `q` | searches company_name, first_name, last_name, email, phone | searches same 5 fields | MATCH |
| `email` | exact match | exact match | MATCH |
| `first_name` | exact match | exact match | MATCH |
| `last_name` | exact match | exact match | MATCH |
| `has_account` | boolean filter | boolean filter | MATCH |
| `offset` | default 0 | default 0 | MATCH |
| `limit` | default 50, max 100 | default 50, max 100 | MATCH |

### Response shape

- Medusa: `AdminCustomerListResponse = { customers: AdminCustomer[], count, offset, limit }`
- toko-rs: `AdminCustomerListResponse { customers: Vec<CustomerWithAddresses>, count, offset, limit }`
- CustomerWithAddresses includes `#[serde(flatten)] Customer` + `addresses: Vec<CustomerAddress>` + default address IDs
- MATCH

### Known divergences

- No `groups` field on customer — requires P2 customer groups module
- No `order_count` computed field — requires P2 aggregation

---

## Cart List — toko-rs Extension (K-11)

Medusa does not have `GET /admin/carts`. This endpoint is a toko-rs extension for operational visibility into abandoned/active carts.

- Filter params: `id`, `customer_id`, `offset`, `limit`
- Response: `{ carts: Vec<CartWithItems>, count, offset, limit }`
- Each cart includes full `items` array

---

## Order Cancel/Complete — Simplified

### Cancel (`POST /admin/orders/:id/cancel`)

| Aspect | Medusa | toko-rs |
|--------|--------|---------|
| Reject already canceled | Yes | Yes (400 InvalidData) |
| Reject completed | Yes | Yes (400 InvalidData) |
| Payment refund | Calls payment provider | Sets payment status to `canceled` (no provider call) |
| Fulfillment check | Rejects if any uncancelled fulfillment | No check (P2 fulfillment module) |
| Sets `canceled_at` | Yes | Yes |
| Response | `{ order: OrderDetail }` | `{ order: OrderWithItems }` |

### Complete (`POST /admin/orders/:id/complete`)

- toko-rs extension — Medusa has no equivalent endpoint
- Rejects already completed or canceled orders (400)
- Sets `status = 'completed'`

---

## Invoice — On-the-fly Generation (Option A)

### Design: No `invoices` table, no PDF

Medusa's tutorial creates a full `invoices` table + PDF generation. For P1, we take a simpler approach:

- **`invoice_config` table** (singleton) — stores issuer company info (name, address, phone, email, logo, notes)
- **Invoice generated at query time** — `GET /admin/orders/:id/invoice` merges `InvoiceConfig` + `OrderWithItems`
- **No persistence** — the invoice IS the order data, enriched with issuer info
- **No PDF** — returns JSON. P2 can add PDF rendering consuming the same JSON structure

### Response shape

```json
{
  "invoice": {
    "invoice_number": "INV-0001",
    "date": "2026-05-01T...",
    "status": "latest",
    "issuer": {
      "company_name": "Toko Sejahtera",
      "company_address": "Jl. Merdeka No. 10, Jakarta",
      "company_phone": "+6281234567890",
      "company_email": "admin@tokosejahtera.com",
      "company_logo": null
    },
    "order": { /* full OrderWithItems */ },
    "notes": null
  }
}
```

### Endpoints

| Endpoint | Method | Path | Notes |
|----------|--------|------|-------|
| Get config | GET | /admin/invoice-config | 404 if not configured |
| Upsert config | POST | /admin/invoice-config | Creates or updates singleton |
| Get invoice | GET | /admin/orders/:id/invoice | 404 if no config or no order |

### Config fields

| Field | Type | Nullable | Notes |
|-------|------|----------|-------|
| `company_name` | text | No | Issuing company name |
| `company_address` | text | No | Company address |
| `company_phone` | text | No | Phone |
| `company_email` | text | No | Email |
| `company_logo` | text | Yes | Logo URL |
| `notes` | text | Yes | Footer notes (e.g., payment terms) |

---

## Test Coverage (30 tests)

| Suite | Tests | Coverage |
|-------|-------|----------|
| customer_test.rs | 8 | admin list with q/email/has_account filters, get by ID, not found |
| cart_test.rs | 5 | admin list all, by customer_id, by id, empty list |
| order_test.rs | 7 | cancel happy path, cancel already-canceled, cancel completed, cancel not found, complete happy path, complete already-completed, complete canceled |
| invoice_test.rs | 10 | config 404 when empty, create via upsert, get after create, partial update, invoice on-the-fly, invoice_number matches display_id, 404 no config, 404 no order, order totals, issuer logo+notes |

---

## Migration Changes

### 002_customers.sql (PG + SQLite)

- Added `CREATE INDEX idx_customers_phone ON customers (phone) WHERE deleted_at IS NULL AND phone IS NOT NULL`
- Checksums changed — requires DB recreation

### 005_payments.sql (PG + SQLite)

- Added `'canceled'` to `status` CHECK constraint
- Checksums changed — requires DB recreation

### 007_invoice_config.sql (PG + SQLite) — NEW

- `CREATE TABLE invoice_config` with company fields + timestamps
- Singleton pattern — upsert creates first row or updates existing

---

## Summary

| Category | Count |
|----------|-------|
| **Endpoints added** | 8 |
| **Checklist entries** | 9 (S-32, S-33, S-34, S-35, D-29, D-30, D-31, L-10, L-11) |
| **Tests added** | 30 |
| **Medusa MATCH** | 3 (customer list, customer get, order cancel) |
| **toko-rs extensions** | 4 (cart list K-11, order complete, invoice config, invoice view K-12) |
| **Known divergences** | 4 (no groups, no fulfillment checks, no refund workflow, JSON invoice vs PDF K-12) |
| **Total test count** | 191 (7 suites) |
