# toko-rs P1: API Compliance and Additions vs Medusa v2

This document records which toko-rs endpoints match MedusaJS v2 (API-compatible), which are custom additions, how the two systems differ in behavior, and which Medusa v2 features are deferred to future phases.

Reference: `vendor/medusa` (v2.15.2 pinned submodule).

---

## 1. Compliant Endpoints (API-compatible with Medusa v2)

The following endpoints are present in both toko-rs and Medusa v2 with matching request/response shapes.

### Store: Products
| Method | Path |
|--------|------|
| GET | `/store/products` |
| GET | `/store/products/:id` |

### Store: Cart
| Method | Path |
|--------|------|
| POST | `/store/carts` |
| GET | `/store/carts/:id` |
| POST | `/store/carts/:id` |
| POST | `/store/carts/:id/line-items` |
| POST | `/store/carts/:id/line-items/:lid` |
| DELETE | `/store/carts/:id/line-items/:lid` |
| POST | `/store/carts/:id/complete` |

### Store: Customers
| Method | Path |
|--------|------|
| POST | `/store/customers` |
| GET | `/store/customers/me` |
| POST | `/store/customers/me` |

### Store: Orders
| Method | Path |
|--------|------|
| GET | `/store/orders` |
| GET | `/store/orders/:id` |

### Admin: Products
| Method | Path |
|--------|------|
| POST | `/admin/products` |
| GET | `/admin/products` |
| GET | `/admin/products/:id` |
| POST | `/admin/products/:id` |
| DELETE | `/admin/products/:id` |
| POST | `/admin/products/:id/variants` |
| GET | `/admin/products/:id/variants` |
| GET | `/admin/products/:id/variants/:vid` |
| POST | `/admin/products/:id/variants/:vid` |
| DELETE | `/admin/products/:id/variants/:vid` |
| GET | `/admin/products/:id/options` |
| POST | `/admin/products/:id/options` |
| GET | `/admin/products/:id/options/:oid` |
| POST | `/admin/products/:id/options/:oid` |
| DELETE | `/admin/products/:id/options/:oid` |

### Admin: Customers
| Method | Path |
|--------|------|
| GET | `/admin/customers` |
| GET | `/admin/customers/:id` |

### Admin: Orders (partial)
| Method | Path | Notes |
|--------|------|-------|
| GET | `/admin/orders` | |
| GET | `/admin/orders/:id` | |
| POST | `/admin/orders/:id/cancel` | |
| POST | `/admin/orders/:id/complete` | with simplified state-machine semantics (no workflow engine) |

---

## 2. toko-rs Additions (not in Medusa v2)

These endpoints exist in toko-rs but have no equivalent in Medusa v2's published OAS.

### Health Check
| Method | Path | Notes |
|--------|------|-------|
| GET | `/health` | Database ping + version. Medusa v2 does not expose a documented health endpoint in its store/admin OAS. |

### Admin: Cart Listing
| Method | Path | Notes |
|--------|------|-------|
| GET | `/admin/carts` | Supports `id` and `customer_id` filters. Medusa v2 uses a draft-orders model for admin-managed carts; there is no direct admin cart list endpoint. |

### Admin: Invoice System
| Method | Path | Notes |
|--------|------|-------|
| GET | `/admin/invoice-config` | Returns issuer company config from environment variables. |
| POST | `/admin/invoice-config` | Read-only — same as GET; returns current env-based config. |
| GET | `/admin/orders/:id/invoice` | On-the-fly invoice generation in JSON (or PDF if rendered by client). No equivalent in Medusa v2. |

### Admin: Event Outbox
| Method | Path | Notes |
|--------|------|-------|
| GET | `/admin/events` | List event outbox. Supports `after` (ISO timestamp cursor), `resource_type`, `unprocessed_only` filters. |
| POST | `/admin/events/:id/mark-processed` | Mark event as processed (consumer ACK). |

### Admin: Webhook Subscriptions
| Method | Path | Notes |
|--------|------|-------|
| POST | `/admin/webhooks` | Register a webhook subscription. Body: `{ url, events: [...], secret }`. Returns `{ webhook }`. |
| GET | `/admin/webhooks` | List all webhook subscriptions. Returns `{ webhooks, count }`. |
| DELETE | `/admin/webhooks/{id}` | Delete a webhook subscription. Returns 204 No Content. |

### Simplified Order Lifecycle Actions
| Method | Path | Notes |
|--------|------|-------|
| POST | `/admin/orders/:id/fulfill` | Sets `fulfillment_status = fulfilled`. Medusa v2 models fulfillment as a separate `/admin/fulfillments` resource with its own lifecycle. |
| POST | `/admin/orders/:id/ship` | Sets `fulfillment_status = shipped`, records `shipped_at`. Medusa v2 handles this via fulfillment status update or shipping notification webhooks. |
| POST | `/admin/orders/:id/capture-payment` | Sets `captured_at` on the payment record. Medusa v2 uses `/admin/payment-collections/:id/payment-sessions/:sid/capture`. |

---

## 3. Behavioral Differences

### Authentication Model
| Aspect | toko-rs | Medusa v2 |
|--------|---------|-----------|
| Customer identity | `X-Customer-Id` request header | JWT access token (Bearer, via `/auth/customer`) |
| Admin identity | No authentication | JWT Bearer token (via `/admin/auth/token`) |
| Session management | Stateless; client holds the customer ID | Refresh-token rotation; session stored server-side |

toko-rs's `X-Customer-Id` header is a deliberate P1 simplification. It provides just enough identity for customer-scoped order listing and profile reads without implementing a full auth system. Medusa v2 uses standards-based JWT with refresh-token rotation.

### Pricing Model
| Aspect | toko-rs | Medusa v2 |
|--------|---------|-----------|
| Price storage | Single `price` integer per variant (minor currency units) | Price lists with money amounts, currency codes, region targeting, and customer group rules |
| Currency | Single `DEFAULT_CURRENCY_CODE` env var | Per-region currency; prices can vary by region and customer segment |
| Price calculation | Direct — price on variant IS the cart price | Calculated at cart time from applicable price list rules |

### Order Lifecycle
| Aspect | toko-rs | Medusa v2 |
|--------|---------|-----------|
| Fulfill | `POST /admin/orders/:id/fulfill` (direct, atomic) | Create fulfillment resource → fulfillment items → fulfillment status |
| Ship | `POST /admin/orders/:id/ship` (direct, sets `shipped_at`) | Update fulfillment shipping status |
| Capture payment | `POST /admin/orders/:id/capture-payment` (direct) | POST to payment-collection/captures with amount |
| Fulfillment model | `fulfillment_status` column on orders | Separate `fulfillments` table with its own items, tracking, and metadata |

toko-rs collapses the fulfillment sub-resource into direct order state transitions. This is intentional for P1 simplicity.

### Order `cart_id` Field (K-13)
| Aspect | toko-rs | Medusa v2 |
|--------|---------|-----------|
| `order.cart_id` in response | Exposed | Omitted from `defaultStoreOrderFields` and `defaultStoreRetrieveOrderFields` |
| Rationale | Required for traceability: toko-rs exposes `GET /admin/carts` (K-11) for operational cart visibility; without `cart_id` on the order response there is no way to navigate from an order back to its originating cart. |

This is an intentional extension (K-13). The `cart_id` column exists on `orders` for idempotency (D-24 / L-9); surfacing it in the response enables `GET /admin/carts?id=<cart_id>` lookups from any order.

### Customer Phone Uniqueness (K-14)
| Aspect | toko-rs | Medusa v2 |
|--------|---------|-----------|
| `customers.phone` uniqueness | Unique partial index `uq_customers_phone ON customers (phone) WHERE deleted_at IS NULL AND phone IS NOT NULL` | No uniqueness constraint on `phone` |
| Duplicate phone POST/PATCH | HTTP 422 `duplicate_error` | HTTP 200 (allowed) |
| Rationale | Required for sapa-rs SMS-keyed integration: customers are looked up by phone, so duplicates would make routing ambiguous. Implemented in migration `007_customers_phone_unique.sql` (PG + SQLite). | n/a |

This is an intentional, permanent divergence (K-14). It is enforced at the DB layer (partial unique index) and the constraint name `uq_customers_phone` is mapped to `AppError::DuplicateError` in `src/customer/repository.rs` (`create` and `update`) so the wire-level response is the standard Medusa-shaped `duplicate_error` payload. Soft-deleted customers (`deleted_at IS NOT NULL`) and `phone = NULL` are excluded from the index, so reusing a phone after a soft-delete or registering multiple customers without phones is still allowed.

---

## 4. Medusa v2 Features Deferred from P1

The following Medusa v2 capabilities are out of scope for toko-rs P1 and planned for future phases:

| Feature | Medusa v2 Path Prefix | Reason Deferred |
|---------|-----------------------|-----------------|
| Admin authentication | `/admin/auth` | Adds complexity; P1 uses open admin endpoints |
| Customer auth (JWT) | `/auth/customer` | X-Customer-Id header is sufficient for P1 |
| Regions | `/admin/regions`, `/store/regions` | Multi-region pricing requires price lists |
| Shipping options | `/admin/shipping-options`, `/store/shipping-options` | No carrier integration in P1 |
| Tax calculation | `/admin/tax-regions`, `/admin/tax-rates` | Flat-price model defers tax |
| Payment providers | `/admin/payment-providers` | Payment captured directly; no provider SDK |
| Inventory management | `/admin/inventory-items`, `/admin/reservations` | Variant stock not tracked in P1 |
| Promotions / discounts | `/admin/promotions`, `/store/promotions` | No discount engine in P1 |
| Product collections | `/admin/collections`, `/store/collections` | Products not grouped in P1 |
| Sales channels | `/admin/sales-channels` | Single-channel in P1 |
| Order returns | `/admin/returns`, `/store/returns` | Return flow deferred |
| Order edits | `/admin/order-edits` | Edit flow deferred |
| Order transfers | `/admin/order-transfers` | Transfer flow deferred |
| Draft orders | `/admin/draft-orders` | Admin cart listing covers the use case partially |
| Customer groups | `/admin/customer-groups` | Segmentation deferred |
| File / image upload | `/admin/uploads` | Images stored as URL strings; no upload service |
| Webhook retry queue / dead-letter | — | Retry on delivery failure and dead-letter storage deferred to P2 |
