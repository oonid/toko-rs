# Seed Data

Spec reference: `openspec/changes/implementation-p1-core-mvp/specs/foundation/spec.md` — "Seed data command" requirement.

## Usage

```bash
# Run seed against configured database
cargo run -- --seed

# Or via Makefile
make seed
```

The `--seed` CLI flag runs the seed function and exits. It does **not** start the HTTP server.

## Design Decisions

### Fixed IDs for idempotency

All seed entities use deterministic, fixed IDs (e.g., `prod_seed_kaos_polos`, `cus_seed_budi`) rather than generated ULIDs. This makes idempotency trivial — the seed function checks if each ID exists before inserting. Running `--seed` multiple times produces the same result.

### Direct SQL, not repositories

The seed function uses raw SQL queries instead of calling repository methods. This follows the module boundary rule — `seed.rs` is shared infrastructure (like `db.rs`), not a domain module. It imports only `crate::db::AppDb` and `crate::error::AppError`. It avoids importing domain repositories while maintaining full control over the insert logic.

### `INSERT OR IGNORE` for sub-entities

Child records (options, option values, variants, variant-option bindings) use `INSERT OR IGNORE` since their existence is guaranteed by the parent check. Parent records (products, customers) use an explicit `SELECT COUNT(*)` check with tracing logs.

### Incrementing variant_rank

Each variant within a product receives an incrementing `variant_rank` (0, 1, 2, ...) matching the order they appear in the seed array. This mirrors the `COALESCE(MAX(variant_rank), -1) + 1` pattern used by `product/repository.rs:add_variant`.

## Seed Data Inventory

### Products (3, all published)

| ID | Title | Handle | Variants | Price Range (IDR) |
|---|---|---|---|---|
| `prod_seed_kaos_polos` | Kaos Polos | `kaos-polos` | 4 (S/M/L/XL) | 75,000 – 80,000 |
| `prod_seed_jeans_slim` | Jeans Slim Fit | `jeans-slim-fit` | 4 (28/30/32/34) | 250,000 – 275,000 |
| `prod_seed_sneakers` | Sneakers Classic | `sneakers-classic` | 5 (39–43) | 450,000 – 475,000 |

Each product has:
- 1 option ("Ukuran" / Size) with 4–5 values
- 4–5 variants with option bindings to the size option
- Status: `published` (visible via `/store/products`)
- Variant SKUs follow the pattern `{PRODUCT_PREFIX}-{SIZE}` (e.g., `KAOS-P-M`, `JEANS-S-30`, `SNKR-41`)

### Entity ID inventory

| Entity type | Count | ID pattern |
|---|---|---|
| Products | 3 | `prod_seed_{product}` |
| Options | 3 | `opt_seed_{product}_size` |
| Option values | 13 (4+4+5) | `optval_seed_{product}_s_{index}` |
| Variants | 13 | `var_seed_{product}_{size}` |
| Variant-option bindings | 13 | `pvo_seed_{product}_{rank}` |
| Customer | 1 | `cus_seed_budi` |

### Customer (1)

| ID | Name | Email | Phone |
|---|---|---|---|
| `cus_seed_budi` | Budi Santoso | budi@example.com | +6281234567890 |

- `has_account = true`
- Can be used with `X-Customer-Id: cus_seed_budi` header for order history endpoints

## Full Commerce Cycle (curl walkthrough)

This section provides a complete, copy-paste-ready curl simulation of every step in a commerce lifecycle using the seed data. Start the server with seed data first:

```bash
DATABASE_URL=sqlite:toko.db cargo run -- --seed && DATABASE_URL=sqlite:toko.db cargo run
```

The server listens on `http://localhost:3000` by default.

### 0. Health check

```bash
curl -s http://localhost:3000/health | jq
```

```json
{
  "status": "ok",
  "database": "connected",
  "version": "0.1.0"
}
```

---

### Step 1: Browse the storefront

A customer arrives and browses published products:

```bash
curl -s http://localhost:3000/store/products | jq
```

```json
{
  "products": [
    {
      "id": "prod_seed_kaos_polos",
      "title": "Kaos Polos",
      "handle": "kaos-polos",
      "status": "published",
      "variants": [
        { "id": "var_seed_kaos_s", "title": "Kaos Polos - S", "sku": "KAOS-P-S", "price": 75000 },
        { "id": "var_seed_kaos_m", "title": "Kaos Polos - M", "sku": "KAOS-P-M", "price": 75000 },
        { "id": "var_seed_kaos_l", "title": "Kaos Polos - L", "sku": "KAOS-P-L", "price": 80000 },
        { "id": "var_seed_kaos_xl", "title": "Kaos Polos - XL", "sku": "KAOS-P-XL", "price": 80000 }
      ]
    },
    {
      "id": "prod_seed_jeans_slim",
      "title": "Jeans Slim Fit",
      ...
    },
    {
      "id": "prod_seed_sneakers",
      "title": "Sneakers Classic",
      ...
    }
  ],
  "count": 3,
  "offset": 0,
  "limit": 20
}
```

### Step 2: View a single product

The customer clicks on Kaos Polos to see full details:

```bash
curl -s http://localhost:3000/store/products/prod_seed_kaos_polos | jq
```

```json
{
  "product": {
    "id": "prod_seed_kaos_polos",
    "title": "Kaos Polos",
    "handle": "kaos-polos",
    "description": "Kaos polos berbahan katun combed 30s, nyaman untuk sehari-hari.",
    "status": "published",
    "options": [
      {
        "id": "opt_seed_kaos_size",
        "title": "Ukuran",
        "values": [
          { "id": "optval_seed_kaos_s_0", "value": "S" },
          { "id": "optval_seed_kaos_s_1", "value": "M" },
          { "id": "optval_seed_kaos_s_2", "value": "L" },
          { "id": "optval_seed_kaos_s_3", "value": "XL" }
        ]
      }
    ],
    "variants": [
      { "id": "var_seed_kaos_m", "title": "Kaos Polos - M", "sku": "KAOS-P-M", "price": 75000 },
      ...
    ]
  }
}
```

### Step 3: Create a cart

The customer decides to buy Kaos Polos size M. First, create a cart:

```bash
curl -s -X POST http://localhost:3000/store/carts \
  -H 'Content-Type: application/json' \
  -d '{"email": "buyer@example.com", "currency_code": "idr"}' | jq
```

```json
{
  "cart": {
    "id": "cart_01JM...",
    "email": "buyer@example.com",
    "currency_code": "idr",
    "items": [],
    "item_total": 0,
    "total": 0
  }
}
```

Save the `cart.id` value for the next steps.

```bash
CART_ID="cart_01JM..."   # paste the actual id
```

### Step 4: Add items to cart

Add Kaos Polos size M (variant `var_seed_kaos_m`, price Rp75,000) with quantity 2:

```bash
curl -s -X POST http://localhost:3000/store/carts/$CART_ID/line-items \
  -H 'Content-Type: application/json' \
  -d '{"variant_id": "var_seed_kaos_m", "quantity": 2}' | jq
```

```json
{
  "cart": {
    "id": "cart_01JM...",
    "items": [
      {
        "id": "citem_01JM...",
        "variant_id": "var_seed_kaos_m",
        "title": "Kaos Polos - M",
        "quantity": 2,
        "unit_price": 75000,
        "snapshot": {
          "product_title": "Kaos Polos",
          "variant_title": "Kaos Polos - M",
          "variant_sku": "KAOS-P-M"
        }
      }
    ],
    "item_total": 150000,
    "total": 150000
  }
}
```

### Step 5: Add a second item (Sneakers)

Add Sneakers size 41 (variant `var_seed_snkr_41`, price Rp450,000):

```bash
curl -s -X POST http://localhost:3000/store/carts/$CART_ID/line-items \
  -H 'Content-Type: application/json' \
  -d '{"variant_id": "var_seed_snkr_41", "quantity": 1}' | jq
```

```json
{
  "cart": {
    "items": [
      { "variant_id": "var_seed_kaos_m", "quantity": 2, "unit_price": 75000 },
      { "variant_id": "var_seed_snkr_41", "quantity": 1, "unit_price": 450000 }
    ],
    "item_total": 600000,
    "total": 600000
  }
}
```

### Step 6: Update item quantity

Change Kaos Polos from 2 to 3. Save the line item ID from step 5 first:

```bash
LINE_ID="citem_01JM..."   # the Kaos Polos line item id

curl -s -X POST http://localhost:3000/store/carts/$CART_ID/line-items/$LINE_ID \
  -H 'Content-Type: application/json' \
  -d '{"quantity": 3}' | jq '.cart.item_total'
```

```json
675000
```

(3 x Rp75,000 + 1 x Rp450,000 = Rp675,000)

### Step 7: Checkout (complete the cart)

Convert the cart into an order:

```bash
curl -s -X POST http://localhost:3000/store/carts/$CART_ID/complete | jq
```

```json
{
  "type": "order",
  "order": {
    "id": "order_01JM...",
    "display_id": 1,
    "status": "pending",
    "currency_code": "idr",
    "items": [
      { "variant_id": "var_seed_kaos_m", "quantity": 3, "unit_price": 75000 },
      { "variant_id": "var_seed_snkr_41", "quantity": 1, "unit_price": 450000 }
    ],
    "item_total": 675000,
    "total": 675000
  },
  "payment": {
    "id": "pay_01JM...",
    "amount": 675000,
    "currency_code": "idr",
    "status": "pending"
  }
}
```

Save the `order.id` for the next step:

```bash
ORDER_ID="order_01JM..."
```

### Step 8: Register as a customer

A new customer signs up:

```bash
curl -s -X POST http://localhost:3000/store/customers \
  -H 'Content-Type: application/json' \
  -d '{"first_name": "Andi", "last_name": "Pratama", "email": "andi@example.com", "phone": "+6281234509876"}' | jq
```

```json
{
  "customer": {
    "id": "cus_01JM...",
    "first_name": "Andi",
    "last_name": "Pratama",
    "email": "andi@example.com",
    "has_account": true
  }
}
```

Save the customer ID:

```bash
CUS_ID="cus_01JM..."
```

### Step 9: Customer creates their own cart + order

Using the seed customer `cus_seed_budi` to buy Jeans Slim size 30:

```bash
curl -s -X POST http://localhost:3000/store/carts \
  -H 'Content-Type: application/json' \
  -d '{"customer_id": "cus_seed_budi", "currency_code": "idr"}' | jq '.cart.id'
```

```bash
CART_ID2="cart_01JM..."   # paste the actual id

curl -s -X POST http://localhost:3000/store/carts/$CART_ID2/line-items \
  -H 'Content-Type: application/json' \
  -d '{"variant_id": "var_seed_jeans_30", "quantity": 1}' | jq '.cart.item_total'
```

```json
250000
```

```bash
curl -s -X POST http://localhost:3000/store/carts/$CART_ID2/complete | jq
```

```json
{
  "type": "order",
  "order": {
    "id": "order_01JM...",
    "display_id": 2,
    "items": [
      { "variant_id": "var_seed_jeans_30", "quantity": 1, "unit_price": 250000 }
    ],
    "item_total": 250000,
    "total": 250000
  },
  "payment": {
    "id": "pay_01JM...",
    "amount": 250000,
    "currency_code": "idr",
    "status": "pending"
  }
}
```

### Step 10: View order history (authenticated)

Budi checks his order history:

```bash
curl -s http://localhost:3000/store/orders \
  -H 'X-Customer-Id: cus_seed_budi' | jq
```

```json
{
  "orders": [
    {
      "id": "order_01JM...",
      "display_id": 2,
      "status": "pending",
      "items": [
        { "variant_id": "var_seed_jeans_30", "quantity": 1, "unit_price": 250000 }
      ],
      "item_total": 250000
    }
  ],
  "count": 1,
  "offset": 0,
  "limit": 20
}
```

### Step 11: View a single order detail

```bash
ORDER_ID2="order_01JM..."   # paste from step 10

curl -s http://localhost:3000/store/orders/$ORDER_ID2 \
  -H 'X-Customer-Id: cus_seed_budi' | jq
```

```json
{
  "order": {
    "id": "order_01JM...",
    "display_id": 2,
    "status": "pending",
    "items": [
      {
        "variant_id": "var_seed_jeans_30",
        "title": "Jeans Slim - 30",
        "quantity": 1,
        "unit_price": 250000
      }
    ],
    "item_total": 250000,
    "total": 250000
  },
  "payment": {
    "id": "pay_01JM...",
    "amount": 250000,
    "currency_code": "idr",
    "status": "pending"
  }
}
```

### Step 12: Update customer profile

```bash
curl -s -X POST http://localhost:3000/store/customers/me \
  -H 'Content-Type: application/json' \
  -H 'X-Customer-Id: cus_seed_budi' \
  -d '{"phone": "+6289999999999"}' | jq
```

```json
{
  "customer": {
    "id": "cus_seed_budi",
    "first_name": "Budi",
    "last_name": "Santoso",
    "email": "budi@example.com",
    "phone": "+6289999999999",
    "has_account": true
  }
}
```

---

## Admin: Product Management

These endpoints manage the product catalog (admin-only, no auth in P1).

### A1: Create a new product (draft)

Create a product with options and variants in one call:

```bash
curl -s -X POST http://localhost:3000/admin/products \
  -H 'Content-Type: application/json' \
  -d '{
    "title": "Hoodie Oversize",
    "description": "Hoodie tebal bahan fleece, cocok untuk musim hujan.",
    "options": [{"title": "Ukuran", "values": ["M", "L", "XL"]}],
    "variants": [
      {"title": "Hoodie - M", "sku": "HOD-M", "price": 185000, "options": {"Ukuran": "M"}},
      {"title": "Hoodie - L", "sku": "HOD-L", "price": 185000, "options": {"Ukuran": "L"}},
      {"title": "Hoodie - XL", "sku": "HOD-XL", "price": 195000, "options": {"Ukuran": "XL"}}
    ]
  }' | jq
```

```json
{
  "product": {
    "id": "prod_01JM...",
    "title": "Hoodie Oversize",
    "handle": "hoodie-oversize",
    "status": "draft",
    "options": [
      {
        "title": "Ukuran",
        "values": [
          { "value": "M" },
          { "value": "L" },
          { "value": "XL" }
        ]
      }
    ],
    "variants": [
      { "id": "prodvar_01JM...", "title": "Hoodie - M", "sku": "HOD-M", "price": 185000 },
      { "id": "prodvar_01JM...", "title": "Hoodie - L", "sku": "HOD-L", "price": 185000 },
      { "id": "prodvar_01JM...", "title": "Hoodie - XL", "sku": "HOD-XL", "price": 195000 }
    ]
  }
}
```

Note: New products are `draft` by default — not visible on `/store/products` until published.

```bash
HOODIE_ID="prod_01JM..."
```

### A2: Create a simple product (no variants)

Not all products need variants:

```bash
curl -s -X POST http://localhost:3000/admin/products \
  -H 'Content-Type: application/json' \
  -d '{"title": "Stiker Logo", "description": "Stiker vinyl waterproof logo toko."}' | jq '.product | {id, title, status, variants}'
```

```json
{
  "id": "prod_01JM...",
  "title": "Stiker Logo",
  "status": "draft",
  "variants": []
}
```

### A3: List all products (admin view)

Includes drafts — unlike the storefront which only shows published:

```bash
curl -s 'http://localhost:3000/admin/products?offset=0&limit=5' | jq '{count, product_titles: [.products[].title]}'
```

```json
{
  "count": 5,
  "product_titles": [
    "Kaos Polos",
    "Jeans Slim Fit",
    "Sneakers Classic",
    "Hoodie Oversize",
    "Stiker Logo"
  ]
}
```

Paginate with `offset` and `limit`:

```bash
curl -s 'http://localhost:3000/admin/products?offset=3&limit=2' | jq '{count, titles: [.products[].title]}'
```

```json
{
  "count": 5,
  "titles": ["Hoodie Oversize", "Stiker Logo"]
}
```

### A4: Get a single product (admin)

```bash
curl -s http://localhost:3000/admin/products/$HOODIE_ID | jq '.product | {id, title, status, options, variants}'
```

### A5: Publish a product

Update the status to make it visible on the storefront:

```bash
curl -s -X POST http://localhost:3000/admin/products/$HOODIE_ID \
  -H 'Content-Type: application/json' \
  -d '{"status": "published"}' | jq '{status: .product.status}'
```

```json
{ "status": "published" }
```

Now verify it appears on the storefront:

```bash
curl -s http://localhost:3000/store/products | jq '[.products[].title]'
```

```json
["Kaos Polos", "Jeans Slim Fit", "Sneakers Classic", "Hoodie Oversize"]
```

### A6: Partial update (change description only)

```bash
curl -s -X POST http://localhost:3000/admin/products/$HOODIE_ID \
  -H 'Content-Type: application/json' \
  -d '{"description": "Hoodie oversize bahan fleece premium, sangat hangat dan nyaman."}' | jq '.product.description'
```

```json
"Hoodie oversize bahan fleece premium, sangat hangat dan nyaman."
```

### A7: Add a new variant to an existing product

Add size XXL to the hoodie after initial creation:

```bash
curl -s -X POST http://localhost:3000/admin/products/$HOODIE_ID/variants \
  -H 'Content-Type: application/json' \
  -d '{"title": "Hoodie - XXL", "sku": "HOD-XXL", "price": 205000, "options": {"Ukuran": "XXL"}}' | jq '{variant_count: (.product.variants | length), new_variant: (.product.variants[-1] | {title, sku, price})}'
```

```json
{
  "variant_count": 4,
  "new_variant": {
    "title": "Hoodie - XXL",
    "sku": "HOD-XXL",
    "price": 205000
  }
}
```

### A8: Soft-delete a product

```bash
curl -s -X DELETE http://localhost:3000/admin/products/$HOODIE_ID | jq
```

```json
{
  "id": "prod_01JM...",
  "object": "product",
  "deleted": true
}
```

The product is no longer visible:

```bash
curl -s http://localhost:3000/admin/products/$HOODIE_ID | jq
```

```json
{ "code": "invalid_request_error", "type": "not_found", "message": "..." }
```

But you can still find it with the `with_deleted` flag:

```bash
curl -s 'http://localhost:3000/admin/products?with_deleted=true' | jq '.count'
```

After deletion, the handle becomes available for reuse:

```bash
curl -s -X POST http://localhost:3000/admin/products \
  -H 'Content-Type: application/json' \
  -d '{"title": "Hoodie Oversize V2"}' | jq '.product.handle'
```

```json
"hoodie-oversize"
```

---

## Cart: Advanced Operations

### C1: Retrieve a cart (GET)

After creating a cart in Step 3, retrieve it later:

```bash
curl -s http://localhost:3000/store/carts/$CART_ID | jq
```

```json
{
  "cart": {
    "id": "cart_01JM...",
    "email": "buyer@example.com",
    "currency_code": "idr",
    "items": [],
    "item_total": 0,
    "total": 0,
    "created_at": "2026-04-09T12:00:00",
    "updated_at": "2026-04-09T12:00:00",
    "completed_at": null,
    "deleted_at": null
  }
}
```

### C2: Update cart email and customer_id

Link a guest cart to a customer or change the email:

```bash
curl -s -X POST http://localhost:3000/store/carts/$CART_ID \
  -H 'Content-Type: application/json' \
  -d '{"email": "budi@example.com", "customer_id": "cus_seed_budi"}' | jq '.cart | {email, customer_id}'
```

```json
{
  "email": "budi@example.com",
  "customer_id": "cus_seed_budi"
}
```

### C3: Remove a line item from the cart

Remove an item without setting quantity to 0:

```bash
curl -s -X DELETE http://localhost:3000/store/carts/$CART_ID/line-items/$LINE_ID | jq '{item_count: (.cart.items | length), item_total: .cart.item_total}'
```

```json
{
  "item_count": 1,
  "item_total": 450000
}
```

### C4: Set quantity to 0 to remove an item

Alternative way to remove — setting quantity to 0 soft-deletes the item:

```bash
curl -s -X POST http://localhost:3000/store/carts/$CART_ID/line-items/$LINE_ID \
  -H 'Content-Type: application/json' \
  -d '{"quantity": 0}' | jq '{item_count: (.cart.items | length), item_total: .cart.item_total}'
```

```json
{
  "item_count": 0,
  "item_total": 0
}
```

### C5: Adding the same variant merges quantity

If you add a variant already in the cart, quantities are merged:

```bash
# First add: quantity 2
curl -s -X POST http://localhost:3000/store/carts/$CART_ID/line-items \
  -H 'Content-Type: application/json' \
  -d '{"variant_id": "var_seed_kaos_m", "quantity": 2}' | jq '.cart.items[0].quantity'
```
```json
2
```

```bash
# Second add of same variant: quantity becomes 2+3=5
curl -s -X POST http://localhost:3000/store/carts/$CART_ID/line-items \
  -H 'Content-Type: application/json' \
  -d '{"variant_id": "var_seed_kaos_m", "quantity": 3}' | jq '.cart.items[0].quantity'
```
```json
5
```

### C6: Create a cart with defaults (no body fields required)

Minimal cart with sensible defaults (currency from config, no email):

```bash
curl -s -X POST http://localhost:3000/store/carts \
  -H 'Content-Type: application/json' \
  -d '{}' | jq '.cart | {id, currency_code, email}'
```

```json
{
  "id": "cart_01JM...",
  "currency_code": "idr",
  "email": null
}
```

### C7: Create a cart with different currency

Override the default currency:

```bash
curl -s -X POST http://localhost:3000/store/carts \
  -H 'Content-Type: application/json' \
  -d '{"currency_code": "usd"}' | jq '.cart.currency_code'
```

```json
"usd"
```

---

## Customer: Profile Operations

### CU1: Get customer profile (authenticated)

Retrieve the logged-in customer's profile using the `X-Customer-Id` header:

```bash
curl -s http://localhost:3000/store/customers/me \
  -H 'X-Customer-Id: cus_seed_budi' | jq
```

```json
{
  "customer": {
    "id": "cus_seed_budi",
    "first_name": "Budi",
    "last_name": "Santoso",
    "email": "budi@example.com",
    "phone": "+6281234567890",
    "has_account": true,
    "created_at": "2026-04-09T00:00:00",
    "updated_at": "2026-04-09T00:00:00"
  }
}
```

### CU2: Update multiple profile fields at once

```bash
curl -s -X POST http://localhost:3000/store/customers/me \
  -H 'Content-Type: application/json' \
  -H 'X-Customer-Id: cus_seed_budi' \
  -d '{"first_name": "Budi Kurniawan", "phone": "+628111222333", "last_name": "Santoso"}' | jq '.customer | {first_name, last_name, phone}'
```

```json
{
  "first_name": "Budi Kurniawan",
  "last_name": "Santoso",
  "phone": "+628111222333"
}
```

---

## Admin: Product Catalog Manipulation

### AP1: Create product with duplicate handle → 422

```bash
curl -s -X POST http://localhost:3000/admin/products \
  -H 'Content-Type: application/json' \
  -d '{"title": "Another Kaos Polos", "handle": "kaos-polos"}' | jq
```

```json
{ "code": "invalid_request_error", "type": "duplicate_error", "message": "Product with handle 'kaos-polos' already exists" }
```

### AP2: Create product with empty title → 400

```bash
curl -s -X POST http://localhost:3000/admin/products \
  -H 'Content-Type: application/json' \
  -d '{"title": ""}' | jq
```

```json
{ "code": "invalid_request_error", "type": "invalid_data", "message": "Validation failed: title cannot be empty" }
```

### AP3: Add variant with duplicate SKU → 422

```bash
curl -s -X POST http://localhost:3000/admin/products/prod_seed_kaos_polos/variants \
  -H 'Content-Type: application/json' \
  -d '{"title": "Dupe", "sku": "KAOS-P-S", "price": 50000}' | jq
```

```json
{ "code": "invalid_request_error", "type": "duplicate_error", "message": "Variant with SKU 'KAOS-P-S' already exists" }
```

### AP4: Get a nonexistent product → 404

```bash
curl -s http://localhost:3000/admin/products/prod_nope | jq
```

```json
{ "code": "invalid_request_error", "type": "not_found", "message": "..." }
```

### AP5: Update a nonexistent product → 404

```bash
curl -s -X POST http://localhost:3000/admin/products/prod_nope \
  -H 'Content-Type: application/json' \
  -d '{"title": "Ghost"}' | jq
```

```json
{ "code": "invalid_request_error", "type": "not_found", "message": "..." }
```

### AP6: Delete a nonexistent product → 404

```bash
curl -s -X DELETE http://localhost:3000/admin/products/prod_nope | jq
```

```json
{ "code": "invalid_request_error", "type": "not_found", "message": "..." }
```

---

## Extended Error Scenarios

**Empty cart checkout:**
```bash
curl -s -X POST http://localhost:3000/store/carts \
  -H 'Content-Type: application/json' -d '{}' | jq '.cart.id'
# CART_ID3="..."
curl -s -X POST http://localhost:3000/store/carts/$CART_ID3/complete | jq
```
```json
{ "code": "invalid_state_error", "type": "unexpected_state", "message": "Cannot complete an empty cart" }
```

**Missing auth header:**
```bash
curl -s http://localhost:3000/store/orders | jq
```
```json
{ "code": "unknown_error", "type": "unauthorized", "message": "Missing X-Customer-Id header" }
```

**Duplicate email registration:**
```bash
curl -s -X POST http://localhost:3000/store/customers \
  -H 'Content-Type: application/json' \
  -d '{"email": "budi@example.com"}' | jq
```
```json
{ "code": "invalid_request_error", "type": "duplicate_error", "message": "Customer with email 'budi@example.com' already exists" }
```

**Invalid email format:**
```bash
curl -s -X POST http://localhost:3000/store/customers \
  -H 'Content-Type: application/json' \
  -d '{"email": "not-an-email", "first_name": "Test"}' | jq
```
```json
{ "code": "invalid_request_error", "type": "invalid_data", "message": "Validation failed: ...email..." }
```

**Invalid quantity (0) when adding item:**
```bash
curl -s -X POST http://localhost:3000/store/carts/$CART_ID/line-items \
  -H 'Content-Type: application/json' \
  -d '{"variant_id": "var_seed_kaos_m", "quantity": 0}' | jq
```
```json
{ "code": "invalid_request_error", "type": "invalid_data", "message": "Validation failed: ...quantity..." }
```

**Nonexistent variant when adding to cart:**
```bash
curl -s -X POST http://localhost:3000/store/carts/$CART_ID/line-items \
  -H 'Content-Type: application/json' \
  -d '{"variant_id": "var_nope", "quantity": 1}' | jq
```
```json
{ "code": "invalid_request_error", "type": "not_found", "message": "..." }
```

**Already-completed cart cannot be updated:**
```bash
# Complete a cart first (see Step 7), then try to change email
curl -s -X POST http://localhost:3000/store/carts/$CART_ID \
  -H 'Content-Type: application/json' \
  -d '{"email": "new@test.com"}' | jq
```
```json
{ "code": "invalid_state_error", "type": "unexpected_state", "message": "Cart is already completed" }
```

**Already-completed cart cannot add items:**
```bash
curl -s -X POST http://localhost:3000/store/carts/$CART_ID/line-items \
  -H 'Content-Type: application/json' \
  -d '{"variant_id": "var_seed_kaos_m", "quantity": 1}' | jq
```
```json
{ "code": "invalid_state_error", "type": "unexpected_state", "message": "Cart is already completed" }
```

**Cart cannot be completed twice:**
```bash
curl -s -X POST http://localhost:3000/store/carts/$CART_ID/complete | jq
```
```json
{ "code": "invalid_state_error", "type": "unexpected_state", "message": "Cart is already completed" }
```

**Nonexistent cart:**
```bash
curl -s http://localhost:3000/store/carts/cart_nope | jq
```
```json
{ "code": "invalid_request_error", "type": "not_found", "message": "..." }
```

**Nonexistent order:**
```bash
curl -s http://localhost:3000/store/orders/order_nope \
  -H 'X-Customer-Id: cus_seed_budi' | jq
```
```json
{ "code": "invalid_request_error", "type": "not_found", "message": "..." }
```

**Customer profile with invalid header (nonexistent customer):**
```bash
curl -s http://localhost:3000/store/customers/me \
  -H 'X-Customer-Id: cus_nope' | jq
```
```json
{ "code": "invalid_request_error", "type": "not_found", "message": "..." }
```

---

### Variant ID reference for curl

| Product | Size | Variant ID | SKU | Price (IDR) |
|---|---|---|---|---|
| Kaos Polos | S | `var_seed_kaos_s` | `KAOS-P-S` | 75,000 |
| Kaos Polos | M | `var_seed_kaos_m` | `KAOS-P-M` | 75,000 |
| Kaos Polos | L | `var_seed_kaos_l` | `KAOS-P-L` | 80,000 |
| Kaos Polos | XL | `var_seed_kaos_xl` | `KAOS-P-XL` | 80,000 |
| Jeans Slim | 28 | `var_seed_jeans_28` | `JEANS-S-28` | 250,000 |
| Jeans Slim | 30 | `var_seed_jeans_30` | `JEANS-S-30` | 250,000 |
| Jeans Slim | 32 | `var_seed_jeans_32` | `JEANS-S-32` | 250,000 |
| Jeans Slim | 34 | `var_seed_jeans_34` | `JEANS-S-34` | 275,000 |
| Sneakers | 39 | `var_seed_snkr_39` | `SNKR-39` | 450,000 |
| Sneakers | 40 | `var_seed_snkr_40` | `SNKR-40` | 450,000 |
| Sneakers | 41 | `var_seed_snkr_41` | `SNKR-41` | 450,000 |
| Sneakers | 42 | `var_seed_snkr_42` | `SNKR-42` | 475,000 |
| Sneakers | 43 | `var_seed_snkr_43` | `SNKR-43` | 475,000 |

---

### Customer ID reference for curl

| Name | ID | Use with |
|---|---|---|
| Budi Santoso | `cus_seed_budi` | `X-Customer-Id` header for order endpoints, `customer_id` in cart creation |
