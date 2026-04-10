# Third Audit: P1 Compatibility Deep-Dive

**Date**: 2026-04-10
**Scope**: Behavioral comparison of toko-rs P1 implementation against Medusa vendor reference (`vendor/medusa/`), focusing on handler semantics, response field nuances, edge cases, and cross-cutting concerns NOT covered in audits 1 (task12) and 2 (task14).

**Methodology**: Line-by-line comparison of Medusa route handlers, validators, workflows, models, and helpers against toko-rs route handlers, types, repositories, and models. Focused on behavioral differences within P1 scope — not missing P2 features.

---

## Findings Summary

| ID | Severity | Category | Finding | Status |
|---|---|---|---|---|
| **S1** | HIGH | Bug | `load_relations` doesn't filter `deleted_at IS NULL` on child tables | Open |
| **S2** | HIGH | Behavior | `deny_unknown_fields` rejects all Medusa SDK fields not in toko-rs schemas | Open |
| **S3** | HIGH | Behavior | Double-soft-delete returns 404 (Medusa returns 200 idempotent) | Open |
| **S4** | MEDIUM | Behavior | Line item dedup ignores metadata (Medusa checks deep-equality) | Open |
| **S5** | MEDIUM | Behavior | Validation errors include `code` field; Medusa Zod errors omit it | Open |
| **S6** | MEDIUM | Behavior | JSON deserialization errors bypass AppError entirely (inconsistent shape) | Open |
| **S7** | MEDIUM | Schema | `images: Vec<String>` vs Medusa's `ProductImage[]` objects | Open |
| **S8** | MEDIUM | Schema | Variant exposes flat `price: i64` that Medusa does not have | Open |
| **S9** | MEDIUM | ID | Order line item prefix `oli` vs Medusa's `ordli` | Open |
| **S10** | MEDIUM | Behavior | Default pagination limit 20 vs Medusa's 50 | Known (design decision) |
| **S11** | LOW | Behavior | Cart completion has no error-with-cart response type `{ type: "cart", cart, error }` | Known (deferred P2) |
| **S12** | LOW | Behavior | No concurrency locking on cart operations | Known (P1 simplification) |
| **S13** | LOW | Behavior | Payment status hardcoded to `"pending"` / `"not_paid"`, never transitions | Known (P1 stub) |
| **S14** | INFO | Match | All response field NAMES match Medusa exactly — zero naming mismatches | Confirmed |
| **S15** | INFO | Match | Customer email correctly excluded from update fields | Confirmed |
| **S16** | INFO | Match | Delete response shape `{ id, object, deleted }` matches Medusa exactly | Confirmed |
| **S17** | INFO | Match | Cart line item soft-delete filtering is correct | Confirmed |
| **S18** | INFO | Match | Order line item soft-delete filtering is correct | Confirmed |
| **S19** | INFO | Match | Cart add_line_item correctly checks variant AND product soft-delete | Confirmed |

---

## HIGH Severity

### S1. `load_relations` doesn't filter `deleted_at IS NULL` on child tables

**File**: `src/product/repository.rs:388-426`

The `load_relations` function queries four child tables without soft-delete filtering:

| Line | Query | Missing filter |
|---|---|---|
| 389-391 | `SELECT * FROM product_options WHERE product_id = $1` | `AND deleted_at IS NULL` |
| 398-400 | `SELECT * FROM product_option_values WHERE option_id = $1` | `AND deleted_at IS NULL` |
| 410-412 | `SELECT * FROM product_variants WHERE product_id = $1` | `AND deleted_at IS NULL` |
| 419-426 | `SELECT ... FROM product_variant_option pvo JOIN product_option_values pov ...` | No filter on either table |

**Impact**: Every product response includes soft-deleted options, option values, variants, and variant-option bindings. While toko-rs doesn't currently expose endpoints to individually soft-delete these children, the DB schema supports it (all tables have `deleted_at` columns). Medusa's MikroORM filter excludes soft-deleted records at the ORM level by default.

**Fix**: Add `AND deleted_at IS NULL` to the three direct child queries and appropriate filters to the join query.

**Medusa reference**: MikroORM's soft-delete filter (`@Filter` with `deleted_at IS NULL`) is applied globally to all relation loads.

---

### S2. `deny_unknown_fields` rejects all Medusa SDK fields not in toko-rs schemas

**Files**: `src/product/types.rs`, `src/cart/types.rs`, `src/customer/types.rs`, `src/order/types.rs`

All input types use `#[serde(deny_unknown_fields)]`. This means any field accepted by Medusa's equivalent Zod schema but not present in toko-rs's struct will cause a 422 error.

**Affected fields by module**:

| Module | Fields rejected that Medusa accepts |
|---|---|
| **Product create/update** | `subtitle`, `is_giftcard`, `discountable`, `images`, `tags`, `categories`, `collection_id`, `type_id`, `sales_channel_id`, `shipping_profile_id`, `external_id`, `weight`, `length`, `height`, `width`, `hs_code`, `mid_code`, `origin_country`, `material`, `additional_data` |
| **Variant create** | `ean`, `upc`, `barcode`, `hs_code`, `mid_code`, `allow_backorder`, `manage_inventory`, `variant_rank`, `inventory_items`, dimensional fields, `prices` (array) |
| **Cart create/update** | `region_id`, `shipping_address`, `billing_address`, `items`, `sales_channel_id`, `promo_codes`, `locale` |
| **Line item add** | Only `variant_id`, `quantity`, `metadata` accepted — Medusa also accepts `title`, `unit_price`, `variant_sku`, etc. |
| **Customer register** | `company_name` (Medusa accepts it, toko-rs does not) |

**Impact**: A Medusa SDK client or any client following the Medusa OAS spec will get 422 on most create/update operations. The `prices` vs `price` difference on variants is the most critical — it's not just a missing field but a different field name.

**Assessment**: This is an intentional design choice (strict input validation), but it makes toko-rs **not a drop-in replacement** for Medusa's API. Clients must be adapted. Documenting this as a known divergence may be preferable to removing `deny_unknown_fields`, since accepting and ignoring fields can mask bugs.

---

### S3. Double-soft-delete returns 404 (Medusa returns 200 idempotent)

**File**: `src/product/repository.rs:261-277`

toko-rs's `soft_delete` uses `WHERE deleted_at IS NULL`. On double-delete, 0 rows are affected, and it returns 404.

**Medusa behavior** (`deleteProductsStep`): Medusa calls `softDeleteProducts(ids)` which silently succeeds on already-deleted products. The DELETE route handler returns `{ id, object: "product", deleted: true }` unconditionally without checking if the product existed.

| Scenario | toko-rs | Medusa |
|---|---|---|
| Delete existing product | 200 `{ id, object, deleted: true }` | 200 `{ id, object, deleted: true }` |
| Delete already-deleted product | **404** | 200 `{ id, object, deleted: true }` |
| Delete nonexistent product | **404** | 200 `{ id, object, deleted: true }` |

**Impact**: DELETE is not idempotent in toko-rs. A client that retries a delete after a network timeout will get a 404, which may trigger error-handling logic. This also affects cart line items and customer soft-deletes if similar patterns are used elsewhere.

**Fix**: Either return success unconditionally (matching Medusa), or check for existence separately and return the standard response regardless of `deleted_at` state.

---

## MEDIUM Severity

### S4. Line item dedup ignores metadata (Medusa checks deep-equality)

**File**: `src/cart/repository.rs:150`

toko-rs dedup query:
```sql
SELECT id, quantity FROM cart_line_items WHERE cart_id = $1 AND variant_id = $2 AND deleted_at IS NULL
```

Medusa dedup logic (`getLineItemActions.ts:95-99`):
```typescript
const metadataMatches = deepEqualObj(existingItem?.metadata, item.metadata)
if (existingItem && metadataMatches) { /* merge */ } else { /* create new */ }
```

**Impact**: Adding the same variant with different metadata merges quantities in toko-rs but creates a separate line item in Medusa. This causes incorrect cart totals and loses metadata distinctions.

**Fix**: Add metadata comparison to the existing-item lookup. If metadata differs, create a new line item instead of merging.

---

### S5. Validation errors include `code` field; Medusa Zod errors omit it

**File**: `src/error.rs:111-115`

toko-rs always returns 3 fields: `{ code, type, message }`.

Medusa's Zod validation errors return only 2 fields: `{ type: "invalid_data", message: "..." }` — the `code` field is absent.

**Impact**: Clients that check for the presence of `code` to distinguish validation errors from other errors will see a difference. Low practical impact since both return `type: "invalid_data"` which is the key field.

---

### S6. JSON deserialization errors bypass AppError entirely

**Files**: `src/lib.rs`, route handlers

toko-rs has no custom `JsonRejection` handler. When axum's `Json<T>` extractor fails (malformed JSON, wrong types), the default rejection fires — producing a response shape like `{ "message": "..." }` with a 400/422 status, which differs from toko-rs's standard `{ code, type, message }` format.

**Medusa handles this** through a global error handler that catches all errors (including parse errors) and formats them into the standard shape.

**Impact**: Malformed JSON bodies produce inconsistent error shapes. This is a real user-facing inconsistency.

**Fix**: Implement a custom `axum::extract::rejection::JsonRejection` handler that wraps the error in `AppError::InvalidData`.

---

### S7. `images: Vec<String>` vs Medusa's `ProductImage[]` objects

**File**: `src/product/models.rs:61`

toko-rs: `images: Vec<String>` (always empty `vec![]`)
Medusa: `images: AdminProductImage[]` where each object has `{ id, url, rank, metadata, created_at, updated_at, deleted_at }`

**Impact**: Clients that try to access `product.images[0].url` will get `undefined` instead of a string (since toko-rs returns strings, not objects). The array is always empty in P1, so this only matters when P2 adds image support.

**Fix now**: Change `Vec<String>` to `Vec<ImageStub>` with `{ url: String }` at minimum, or the full `{ id, url, rank }` shape. This prevents a breaking type change later.

---

### S8. Variant exposes flat `price: i64` that Medusa does not have

**File**: `src/product/models.rs:74-79`

`ProductVariantWithOptions` uses `#[serde(flatten)]` on the variant, exposing both `price: i64` and `calculated_price: { ... }` in the response.

Medusa's `BaseProductVariant` has NO flat `price` field. Pricing is only available through:
- Store: `calculated_price: BaseCalculatedPriceSet`
- Admin: `prices: AdminPrice[]` (multi-currency array)

**Impact**: The extra `price` field is not harmful (clients can ignore it) but it's a toko-rs extension that Medusa clients won't expect. More importantly, the admin variant response lacks the `prices` array that Medusa returns.

---

### S9. Order line item prefix `oli` vs Medusa's `ordli`

**File**: `src/order/repository.rs:71`

toko-rs: `generate_entity_id("oli")` → IDs like `oli_01HXYZ...`
Medusa: `model.id({ prefix: "ordli" })` → IDs like `ordli_01HXYZ...`

**Impact**: Cosmetic for internal use. Any ID-pattern-matching logic in clients or integration tests will break. Other prefixes (product, variant, cart, customer) match.

**Known prefix mismatches**:

| Entity | toko-rs | Medusa | Match? |
|---|---|---|---|
| Order line item | `oli` | `ordli` | NO |
| Product variant option | `pvo` | (join table, no prefix) | N/A |
| All others | match | match | YES |

---

## LOW Severity / Known Divergences

### S10. Default pagination limit 20 vs Medusa's 50

toko-rs defaults to 20 (changed in Task 4e.2). Medusa defaults to 50 (`createFindParams({ limit: 50 })`).

**Assessment**: Already documented as a design decision. Clients that don't specify `limit` will get fewer results per page.

### S11. Cart completion has no error-with-cart response type

Medusa returns `{ type: "cart", cart: <cart>, error: { message, name, type } }` for payment-related soft failures during completion. toko-rs only has the success path `{ type: "order", order }`.

**Assessment**: Deferred to P2 (requires `payment_session` table).

### S12. No concurrency locking on cart operations

Medusa uses `acquireLockStep` / `releaseLockStep` (distributed locks) around add-to-cart. toko-rs relies on database transaction isolation.

**Assessment**: Acceptable for P1 single-binary deployment.

### S13. Payment status hardcoded, never transitions

toko-rs: `PaymentRecord.status` always `"pending"`, order `payment_status` always `"not_paid"`, `fulfillment_status` always `"not_fulfilled"`.

Medusa: These are computed dynamically from underlying payment collections, captures, refunds, and fulfillments.

**Assessment**: P1 stub. The stub values (`"not_paid"`, `"not_fulfilled"`) are valid Medusa enum values.

---

## Confirmed Matches (Positive Findings)

### S14. Zero response field name mismatches

Every field name in toko-rs that has a Medusa counterpart uses the identical name:
- All 22 computed total fields on cart/order match exactly
- `calculated_price.calculated_amount`, `original_amount`, `is_calculated_price_tax_inclusive` all match
- `payment_status`, `fulfillment_status`, `fulfillments`, `shipping_methods` match
- `addresses`, `default_billing_address_id`, `default_shipping_address_id` match
- All `CustomerAddress` fields match Medusa's `BaseCustomerAddress`

### S15. Customer email correctly excluded from update

`UpdateCustomerInput` does not include `email`. `deny_unknown_fields` rejects email in update requests. Matches Medusa's `StoreUpdateCustomer` schema.

### S16. Delete response shape matches exactly

`{ id, object: "product", deleted: true }` matches Medusa's `DeleteResponse` (ignoring the double-delete divergence in S3).

### S17-S19. Soft-delete filtering is correct everywhere except `load_relations`

Cart `get_cart`, cart `add_line_item`, order `find_by_id`, order `list_by_customer` all correctly filter `deleted_at IS NULL`. The ONLY gap is product `load_relations` (S1).

---

## Error Handler Mapping Verification

Full verification against `vendor/medusa/packages/core/framework/src/http/middlewares/error-handler.ts`:

| toko-rs Variant | HTTP | type | code | Medusa mapping | Match? |
|---|---|---|---|---|---|
| `NotFound` | 404 | `not_found` | `invalid_request_error` | `NOT_FOUND` → 404, code passthrough | YES |
| `InvalidData` | 400 | `invalid_data` | `invalid_request_error` | `INVALID_DATA` → 400, code passthrough | YES* |
| `DuplicateError` | 422 | `duplicate_error` | `invalid_request_error` | `DUPLICATE_ERROR` → 422, code overridden to `invalid_request_error` | YES |
| `Forbidden` | 403 | `forbidden` | `invalid_state_error` | `FORBIDDEN` → 403, code passthrough | YES |
| `Conflict` | 409 | `conflict` | `invalid_state_error` | `CONFLICT` → 409, code overridden to `invalid_state_error` | YES |
| `Unauthorized` | 401 | `unauthorized` | `unknown_error` | `UNAUTHORIZED` → 401, code passthrough | YES |
| `UnexpectedState` | 500 | `unexpected_state` | `invalid_state_error` | `UNEXPECTED_STATE` → 500, code passthrough | YES |
| `DatabaseError` | 500 | `database_error` | `api_error` | `DB_ERROR` → 500, code overridden to `api_error` | YES |
| `MigrationError` | 500 | `database_error` | `api_error` | Same as DB_ERROR | YES |

*Note S5: Medusa's Zod validation errors omit `code`, but toko-rs includes it. This is the one remaining shape difference in otherwise-correct error mapping.

### PG Error Code Pre-processing Verification

Medusa's `formatException()` in the error handler pre-processes raw PG errors before they reach the main error handler:

| PG Code | Medusa converts to | toko-rs `map_db_constraint()` | Match? |
|---|---|---|---|
| `23505` | `DUPLICATE_ERROR` | `DuplicateError` | YES |
| `23503` | `NOT_FOUND` with parsed detail | `NotFound` | YES |
| `40001` | `CONFLICT` | (not mapped) | NO |
| `23502` | `INVALID_DATA` | `InvalidData` | YES |

**Gap**: PG error code `40001` (serialization failure) is mapped to `CONFLICT` in Medusa but not handled in toko-rs. This occurs under concurrent transaction conflicts and would surface as a generic `DatabaseError` (500) instead of a proper `Conflict` (409).

---

## Snapshot Field Completeness

### Cart Line Item Snapshot

Medusa captures 20+ denormalized fields at add-time. toko-rs captures 3 in a JSON blob:

| Medusa field | toko-rs equivalent |
|---|---|
| `product_title` | In snapshot JSON |
| `variant_title` | In snapshot JSON |
| `variant_sku` | In snapshot JSON |
| `subtitle` (variant title) | **Missing** |
| `thumbnail` | **Missing** |
| `product_description` | **Missing** |
| `product_subtitle` | **Missing** |
| `product_type` | **Missing** |
| `product_type_id` | **Missing** |
| `product_collection` | **Missing** |
| `product_handle` | **Missing** |
| `variant_barcode` | **Missing** |
| `variant_option_values` | **Missing** |
| `compare_at_unit_price` | **Missing** |
| `is_giftcard` | **Missing** |

**Assessment**: The `snapshot` JSON column is extensible — adding fields is a code-only change (no migration needed). However, existing carts in production would have incomplete snapshots. For P1 this is acceptable since there are no historical carts to worry about.

---

## Recommendations

### Fix in P1 (before considering Task 17 complete)

1. **S1**: Add `AND deleted_at IS NULL` to all 4 child-table queries in `load_relations`
2. **S6**: Add custom JSON rejection handler for consistent error shapes

### Fix in P1 (recommended but not blocking)

3. **S3**: Make DELETE idempotent (return 200 instead of 404 on double-delete)
4. **S4**: Add metadata comparison to line item dedup
5. **S7**: Change `images: Vec<String>` to `Vec<ImageStub>` with `{ url }` shape

### Document as known P1 divergence (no fix needed)

6. **S2**: `deny_unknown_fields` — intentional strict validation
7. **S8**: Extra `price` field on variants — harmless extension
8. **S9**: Order line item prefix `oli` vs `ordli` — cosmetic
9. **S10**: Pagination limit 20 vs 50 — already documented
10. **S5**: Validation error includes `code` — minor shape difference
11. **PG 40001**: Serialization failure not mapped to Conflict — rare edge case
