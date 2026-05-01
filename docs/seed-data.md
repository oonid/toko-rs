# Seed Data

Spec reference: `openspec/changes/implementation-p1-core-mvp/specs/foundation/spec.md` — "Seed data command" requirement.

## Usage

```bash
# Start PostgreSQL via Docker Compose
docker compose up -d

# Run seed against configured database
DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/toko cargo run -- --seed

# Start the server
DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/toko cargo run
```

Or via Makefile:

```bash
make seed   # seeds the database
make dev    # starts the server
```

The `--seed` CLI flag runs the seed function and exits. It does **not** start the HTTP server.

The server listens on `http://localhost:3000` by default.

## Design Decisions

### Fixed IDs for idempotency

All seed entities use deterministic, fixed IDs (e.g., `prod_seed_kaos_polos`, `cus_seed_budi`) rather than generated ULIDs. This makes idempotency trivial — the seed function checks if each ID exists before inserting. Running `--seed` multiple times produces the same result.

### Direct SQL, not repositories

The seed function uses raw SQL queries instead of calling repository methods. This follows the module boundary rule — `seed.rs` is shared infrastructure (like `db.rs`), not a domain module. It imports only `crate::db::AppDb` and `crate::error::AppError`. It avoids importing domain repositories while maintaining full control over the insert logic.

### `INSERT ... ON CONFLICT DO NOTHING` for sub-entities

Child records (options, option values, variants, variant-option bindings) use `ON CONFLICT (id) DO NOTHING` since their existence is guaranteed by the parent check. Parent records (products, customers) use an explicit `SELECT COUNT(*)` check with tracing logs.

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

| ID | Name | Email | Phone | Company |
|---|---|---|---|---|
| `cus_seed_budi` | Budi Santoso | budi@example.com | +6281234567890 | Toko Budi Sejahtera |

- `has_account = true`
- Can be used with `X-Customer-Id: cus_seed_budi` header for order history endpoints

## Full Commerce Cycle (curl walkthrough)

This section provides a complete, copy-paste-ready curl simulation of every step in a commerce lifecycle using the seed data. JSON responses show key fields — many endpoints return additional computed total fields not shown here for brevity.

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
      "description": "Kaos polos berbahan katun combed 30s, nyaman untuk sehari-hari.",
      "subtitle": null,
      "status": "published",
      "thumbnail": "https://example.com/kaos-polos.jpg",
      "metadata": null,
      "is_giftcard": false,
      "discountable": true,
      "collection_id": null,
      "type_id": null,
      "created_at": "2026-04-28T...",
      "updated_at": "2026-04-28T...",
      "deleted_at": null,
      "options": [
        {
          "id": "opt_seed_kaos_size",
          "product_id": "prod_seed_kaos_polos",
          "title": "Ukuran",
          "metadata": null,
          "created_at": "...",
          "updated_at": "...",
          "values": [
            { "id": "optval_seed_kaos_s_0", "option_id": "opt_seed_kaos_size", "value": "S", "metadata": null, "created_at": "...", "updated_at": "..." },
            { "id": "optval_seed_kaos_s_1", "option_id": "opt_seed_kaos_size", "value": "M", "metadata": null, "created_at": "...", "updated_at": "..." },
            { "id": "optval_seed_kaos_s_2", "option_id": "opt_seed_kaos_size", "value": "L", "metadata": null, "created_at": "...", "updated_at": "..." },
            { "id": "optval_seed_kaos_s_3", "option_id": "opt_seed_kaos_size", "value": "XL", "metadata": null, "created_at": "...", "updated_at": "..." }
          ]
        }
      ],
      "variants": [
        {
          "id": "var_seed_kaos_s",
          "product_id": "prod_seed_kaos_polos",
          "title": "Kaos Polos - S",
          "sku": "KAOS-P-S",
          "thumbnail": null,
          "price": 75000,
          "variant_rank": 0,
          "metadata": null,
          "created_at": "...",
          "updated_at": "...",
          "options": [
            { "id": "optval_seed_kaos_s_0", "value": "S", "option": { "id": "opt_seed_kaos_size", "title": "Ukuran" } }
          ],
          "calculated_price": {
            "calculated_amount": 75000,
            "original_amount": 75000,
            "is_calculated_price_tax_inclusive": false,
            "currency_code": "idr"
          }
        }
      ],
      "images": []
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
  "limit": 50
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
    "subtitle": null,
    "status": "published",
    "thumbnail": "https://example.com/kaos-polos.jpg",
    "metadata": null,
    "is_giftcard": false,
    "discountable": true,
    "collection_id": null,
    "type_id": null,
    "created_at": "...",
    "updated_at": "...",
    "deleted_at": null,
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
      {
        "id": "var_seed_kaos_m",
        "title": "Kaos Polos - M",
        "sku": "KAOS-P-M",
        "thumbnail": null,
        "price": 75000,
        "variant_rank": 1,
        "options": [ { "id": "optval_seed_kaos_s_1", "value": "M", "option": { "id": "opt_seed_kaos_size", "title": "Ukuran" } } ],
        "calculated_price": { "calculated_amount": 75000, "original_amount": 75000, "is_calculated_price_tax_inclusive": false, "currency_code": "idr" }
      }
    ],
    "images": []
  }
}
```

### Step 3: Create a cart

The customer decides to buy Kaos Polos size M. First, create a cart:

```bash
curl -s -X POST http://localhost:3000/store/carts \
  -H 'Content-Type: application/json' \
  -d '{"email": "buyer@example.com", "currency_code": "idr"}' | jq '.cart | {id, email, currency_code, items, item_total, total}'
```

```json
{
  "id": "cart_01KQ...",
  "email": "buyer@example.com",
  "currency_code": "idr",
  "items": [],
  "item_total": 0,
  "total": 0
}
```

Save the `cart.id` value for the next steps.

```bash
CART_ID="cart_01KQ..."   # paste the actual id
```

### Step 4: Add items to cart

Add Kaos Polos size M (variant `var_seed_kaos_m`, price Rp75,000) with quantity 2:

```bash
curl -s -X POST http://localhost:3000/store/carts/$CART_ID/line-items \
  -H 'Content-Type: application/json' \
  -d '{"variant_id": "var_seed_kaos_m", "quantity": 2}' | jq '.cart.items[0] | {id, title, quantity, unit_price, compare_at_unit_price, variant_id, thumbnail, product_title, variant_sku, variant_option_values}'
```

```json
{
  "id": "cali_01KQ...",
  "title": "Kaos Polos",
  "quantity": 2,
  "unit_price": 75000,
  "compare_at_unit_price": null,
  "variant_id": "var_seed_kaos_m",
  "thumbnail": "https://example.com/kaos-polos.jpg",
  "product_title": "Kaos Polos",
  "variant_sku": "KAOS-P-M",
  "variant_option_values": { "Ukuran": "M" }
}
```

### Step 5: Add a second item (Sneakers)

Add Sneakers size 41 (variant `var_seed_snkr_41`, price Rp450,000):

```bash
curl -s -X POST http://localhost:3000/store/carts/$CART_ID/line-items \
  -H 'Content-Type: application/json' \
  -d '{"variant_id": "var_seed_snkr_41", "quantity": 1}' | jq '.cart | {item_total, total}'
```

```json
{
  "item_total": 600000,
  "total": 600000
}
```

### Step 6: Update item quantity

Change Kaos Polos from 2 to 3. Save the line item ID from step 4 first:

```bash
LINE_ID="cali_01KQ..."   # the Kaos Polos line item id

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
    "id": "order_01KQ...",
    "display_id": 1,
    "cart_id": "cart_01KQ...",
    "customer_id": null,
    "email": "buyer@example.com",
    "currency_code": "idr",
    "status": "pending",
    "shipping_address": null,
    "billing_address": null,
    "metadata": null,
    "canceled_at": null,
    "created_at": "...",
    "updated_at": "...",
    "items": [
      {
        "id": "ordli_01KQ...",
        "order_id": "order_01KQ...",
        "title": "Kaos Polos",
        "quantity": 3,
        "unit_price": 75000,
        "compare_at_unit_price": null,
        "variant_id": "var_seed_kaos_m",
        "product_id": "prod_seed_kaos_polos",
        "metadata": null,
        "requires_shipping": true,
        "is_discountable": true,
        "is_tax_inclusive": false,
        "product_title": "Kaos Polos",
        "product_handle": "kaos-polos",
        "variant_sku": "KAOS-P-M",
        "variant_title": "Kaos Polos - M",
        "variant_option_values": { "Ukuran": "M" },
        "thumbnail": "https://example.com/kaos-polos.jpg",
        "is_giftcard": false
      },
      {
        "id": "ordli_01KQ...",
        "title": "Sneakers Classic",
        "quantity": 1,
        "unit_price": 450000,
        "variant_id": "var_seed_snkr_41",
        "product_id": "prod_seed_sneakers"
      }
    ],
    "item_total": 675000,
    "total": 675000,
    "payment_status": "not_paid",
    "fulfillment_status": "not_fulfilled",
    "fulfillments": [],
    "shipping_methods": []
  }
}
```

Note: Cart complete returns `{ type: "order", order: {...} }` only — no top-level `payment` field.

Save the `order.id` for the next step:

```bash
ORDER_ID="order_01KQ..."
```

### Step 8: Register as a customer

A new customer signs up:

```bash
curl -s -X POST http://localhost:3000/store/customers \
  -H 'Content-Type: application/json' \
  -d '{"first_name": "Andi", "last_name": "Pratama", "email": "andi@example.com", "phone": "+6281234509876"}' | jq '.customer | {id, first_name, last_name, email, has_account, company_name, created_by, addresses}'
```

```json
{
  "id": "cus_01KQ...",
  "first_name": "Andi",
  "last_name": "Pratama",
  "email": "andi@example.com",
  "has_account": true,
  "company_name": null,
  "created_by": null,
  "addresses": []
}
```

Save the customer ID:

```bash
CUS_ID="cus_01KQ..."
```

### Step 9: Customer creates their own cart + order

Using the seed customer `cus_seed_budi` to buy Jeans Slim size 30:

```bash
curl -s -X POST http://localhost:3000/store/carts \
  -H 'Content-Type: application/json' \
  -d '{"customer_id": "cus_seed_budi", "currency_code": "idr"}' | jq '.cart.id'
```

```bash
CART_ID2="cart_01KQ..."   # paste the actual id

curl -s -X POST http://localhost:3000/store/carts/$CART_ID2/line-items \
  -H 'Content-Type: application/json' \
  -d '{"variant_id": "var_seed_jeans_30", "quantity": 1}' | jq '.cart.item_total'
```

```json
250000
```

```bash
curl -s -X POST http://localhost:3000/store/carts/$CART_ID2/complete | jq '{type, display_id: .order.display_id, item_total: .order.item_total}'
```

```json
{
  "type": "order",
  "display_id": 2,
  "item_total": 250000
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
      "id": "order_01KQ...",
      "display_id": 2,
      "status": "pending",
      "currency_code": "idr",
      "items": [
        { "variant_id": "var_seed_jeans_30", "quantity": 1, "unit_price": 250000 }
      ],
      "item_total": 250000
    }
  ],
  "count": 1,
  "offset": 0,
  "limit": 50
}
```

### Step 11: View a single order detail

```bash
ORDER_ID2="order_01KQ..."   # paste from step 10

curl -s http://localhost:3000/store/orders/$ORDER_ID2 \
  -H 'X-Customer-Id: cus_seed_budi' | jq '.order | {id, display_id, status, item_total, total, payment_status, fulfillment_status}'
```

```json
{
  "id": "order_01KQ...",
  "display_id": 2,
  "status": "pending",
  "item_total": 250000,
  "total": 250000,
  "payment_status": "not_paid",
  "fulfillment_status": "not_fulfilled"
}
```

### Step 12: Update customer profile

```bash
curl -s -X POST http://localhost:3000/store/customers/me \
  -H 'Content-Type: application/json' \
  -H 'X-Customer-Id: cus_seed_budi' \
  -d '{"phone": "+6289999999999"}' | jq '.customer | {id, first_name, last_name, email, phone, has_account, company_name, created_by, addresses}'
```

```json
{
  "id": "cus_seed_budi",
  "first_name": "Budi",
  "last_name": "Santoso",
  "email": "budi@example.com",
  "phone": "+6289999999999",
  "has_account": true,
  "company_name": "Toko Budi Sejahtera",
  "created_by": null,
  "addresses": []
}
```

You can also update the email:

```bash
curl -s -X POST http://localhost:3000/store/customers/me \
  -H 'Content-Type: application/json' \
  -H 'X-Customer-Id: cus_seed_budi' \
  -d '{"email": "budi.new@example.com"}' | jq '.customer.email'
```

```json
"budi.new@example.com"
```

---

## Admin: Product Management

These endpoints manage the product catalog (admin-only, no auth in P1).

### A1: Create a new product (draft) with images

Create a product with options, variants, and images in one call:

```bash
curl -s -X POST http://localhost:3000/admin/products \
  -H 'Content-Type: application/json' \
  -d '{
    "title": "Hoodie Oversize",
    "description": "Hoodie tebal bahan fleece, cocok untuk musim hujan.",
    "images": [{"url": "https://example.com/hoodie-front.jpg"}, {"url": "https://example.com/hoodie-back.jpg"}],
    "options": [{"title": "Ukuran", "values": ["M", "L", "XL", "XXL"]}],
    "variants": [
      {"title": "Hoodie - M", "sku": "HOD-M", "price": 185000, "thumbnail": "https://example.com/hoodie-m.jpg", "options": {"Ukuran": "M"}},
      {"title": "Hoodie - L", "sku": "HOD-L", "price": 185000, "options": {"Ukuran": "L"}},
      {"title": "Hoodie - XL", "sku": "HOD-XL", "price": 195000, "options": {"Ukuran": "XL"}}
    ]
  }' | jq '.product | {id, title, handle, status, images, variants: [.variants[] | {title, sku, price, thumbnail, calculated_price}], options: [.options[] | {title, values: [.values[].value]}]}'
```

```json
{
  "id": "prod_01KQ...",
  "title": "Hoodie Oversize",
  "handle": "hoodie-oversize",
  "status": "draft",
  "images": [
    { "id": "img_01KQ...", "url": "https://example.com/hoodie-front.jpg", "rank": 0, "metadata": null, "created_at": "...", "updated_at": "..." },
    { "id": "img_01KQ...", "url": "https://example.com/hoodie-back.jpg", "rank": 1, "metadata": null, "created_at": "...", "updated_at": "..." }
  ],
  "variants": [
    { "title": "Hoodie - M", "sku": "HOD-M", "price": 185000, "thumbnail": "https://example.com/hoodie-m.jpg", "calculated_price": { "calculated_amount": 185000, "original_amount": 185000, "is_calculated_price_tax_inclusive": false, "currency_code": "idr" } },
    { "title": "Hoodie - L", "sku": "HOD-L", "price": 185000, "thumbnail": null, "calculated_price": { "calculated_amount": 185000, "original_amount": 185000, "is_calculated_price_tax_inclusive": false, "currency_code": "idr" } },
    { "title": "Hoodie - XL", "sku": "HOD-XL", "price": 195000, "thumbnail": null, "calculated_price": { "calculated_amount": 195000, "original_amount": 195000, "is_calculated_price_tax_inclusive": false, "currency_code": "idr" } }
  ],
  "options": [
    { "title": "Ukuran", "values": ["M", "L", "XL"] }
  ]
}
```

Note: New products are `draft` by default — not visible on `/store/products` until published.

```bash
HOODIE_ID="prod_01KQ..."
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
  "id": "prod_01KQ...",
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
curl -s http://localhost:3000/admin/products/$HOODIE_ID | jq '.product | {id, title, status, options, variants, images}'
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

### A6: Partial update (change description and images)

```bash
curl -s -X POST http://localhost:3000/admin/products/$HOODIE_ID \
  -H 'Content-Type: application/json' \
  -d '{"description": "Hoodie oversize bahan fleece premium.", "images": [{"url": "https://example.com/hoodie-v2.jpg"}]}' | jq '.product | {description, images: [.images[].url]}'
```

```json
{
  "description": "Hoodie oversize bahan fleece premium.",
  "images": ["https://example.com/hoodie-v2.jpg"]
}
```

Note: Updating `images` replaces all existing images (soft-deletes old, inserts new).

### A7: Add a new variant to an existing product

Add size XXL to the hoodie after initial creation:

```bash
curl -s -X POST http://localhost:3000/admin/products/$HOODIE_ID/variants \
  -H 'Content-Type: application/json' \
  -d '{"title": "Hoodie - XXL", "sku": "HOD-XXL", "price": 205000, "thumbnail": "https://example.com/hoodie-xxl.jpg", "options": {"Ukuran": "XXL"}}' | jq '{variant_count: (.product.variants | length), new_variant: (.product.variants[-1] | {title, sku, price, thumbnail})}'
```

```json
{
  "variant_count": 4,
  "new_variant": {
    "title": "Hoodie - XXL",
    "sku": "HOD-XXL",
    "price": 205000,
    "thumbnail": "https://example.com/hoodie-xxl.jpg"
  }
}
```

### A8: Soft-delete a product

```bash
curl -s -X DELETE http://localhost:3000/admin/products/$HOODIE_ID | jq
```

```json
{
  "id": "prod_01KQ...",
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

## Admin: Option CRUD

Product options can be managed independently of the parent product. These endpoints handle create, read, update, and delete for options.

### AO1: List options for a product

```bash
curl -s http://localhost:3000/admin/products/prod_seed_kaos_polos/options | jq
```

```json
{
  "product_options": [
    {
      "id": "opt_seed_kaos_size",
      "product_id": "prod_seed_kaos_polos",
      "title": "Ukuran",
      "metadata": null,
      "created_at": "...",
      "updated_at": "...",
      "values": [
        { "id": "optval_seed_kaos_s_2", "option_id": "opt_seed_kaos_size", "value": "L", "metadata": null, "created_at": "...", "updated_at": "..." },
        { "id": "optval_seed_kaos_s_1", "option_id": "opt_seed_kaos_size", "value": "M", "metadata": null, "created_at": "...", "updated_at": "..." },
        { "id": "optval_seed_kaos_s_0", "option_id": "opt_seed_kaos_size", "value": "S", "metadata": null, "created_at": "...", "updated_at": "..." },
        { "id": "optval_seed_kaos_s_3", "option_id": "opt_seed_kaos_size", "value": "XL", "metadata": null, "created_at": "...", "updated_at": "..." }
      ]
    }
  ],
  "count": 1,
  "offset": 0,
  "limit": 50
}
```

### AO2: Get a single option

```bash
curl -s http://localhost:3000/admin/products/prod_seed_kaos_polos/options/opt_seed_kaos_size | jq '.product_option | {id, title, values: [.values[].value]}'
```

```json
{
  "id": "opt_seed_kaos_size",
  "title": "Ukuran",
  "values": ["L", "M", "S", "XL"]
}
```

### AO3: Create a new option on an existing product

Add a "Color" option to Kaos Polos after initial creation:

```bash
curl -s -X POST http://localhost:3000/admin/products/prod_seed_kaos_polos/options \
  -H 'Content-Type: application/json' \
  -d '{"title": "Warna", "values": ["Hitam", "Putih"]}' | jq '.product.options[] | select(.title == "Warna") | {title, values: [.values[].value]}'
```

```json
{
  "title": "Warna",
  "values": ["Hitam", "Putih"]
}
```

Note: Returns `{ product: ProductWithRelations }` — the full updated product.

### AO4: Update an option title

```bash
curl -s -X POST http://localhost:3000/admin/products/prod_seed_kaos_polos/options/opt_seed_kaos_size \
  -H 'Content-Type: application/json' \
  -d '{"title": "Ukuran (Size)"}' | jq '.product.options[] | select(.id == "opt_seed_kaos_size") | .title'
```

```json
"Ukuran (Size)"
```

### AO5: Delete an option

```bash
OPT_ID=$(curl -s http://localhost:3000/admin/products/prod_seed_kaos_polos/options | jq -r '.product_options[0].id')

curl -s -X DELETE http://localhost:3000/admin/products/prod_seed_kaos_polos/options/$OPT_ID | jq
```

```json
{
  "id": "opt_seed_kaos_size",
  "object": "product_option",
  "deleted": true,
  "parent": { "id": "prod_seed_kaos_polos", "title": "Kaos Polos", ... }
}
```

---

## Cart: Advanced Operations

### C1: Retrieve a cart (GET)

After creating a cart in Step 3, retrieve it later:

```bash
curl -s http://localhost:3000/store/carts/$CART_ID | jq '.cart | {id, email, currency_code, items, item_total, total, completed_at}'
```

```json
{
  "id": "cart_01KQ...",
  "email": "buyer@example.com",
  "currency_code": "idr",
  "items": [],
  "item_total": 0,
  "total": 0,
  "completed_at": null
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

Remove an item with DELETE:

```bash
curl -s -X DELETE http://localhost:3000/store/carts/$CART_ID/line-items/$LINE_ID | jq '{id: .id, object: .object, deleted: .deleted, item_count: (.cart.items | length), item_total: .cart.item_total}'
```

```json
{
  "id": "cali_01KQ...",
  "object": "line-item",
  "deleted": true,
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
  "id": "cart_01KQ...",
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
  -H 'X-Customer-Id: cus_seed_budi' | jq '.customer | {id, first_name, last_name, email, phone, company_name, has_account, created_by, addresses}'
```

```json
{
  "id": "cus_seed_budi",
  "first_name": "Budi",
  "last_name": "Santoso",
  "email": "budi@example.com",
  "phone": "+6281234567890",
  "company_name": "Toko Budi Sejahtera",
  "has_account": true,
  "created_by": null,
  "addresses": []
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

### CU3: Update email

```bash
curl -s -X POST http://localhost:3000/store/customers/me \
  -H 'Content-Type: application/json' \
  -H 'X-Customer-Id: cus_seed_budi' \
  -d '{"email": "budi.new@example.com"}' | jq '.customer.email'
```

```json
"budi.new@example.com"
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

## Admin Operations (Task 32)

### AC1 — List customers

Filter by search query, email, name, or account status:

```bash
curl -s http://localhost:3000/admin/customers | jq
```

```json
{
  "customers": [
    {
      "id": "cus_seed_budi",
      "first_name": "Budi",
      "last_name": "Santoso",
      "email": "budi@example.com",
      "phone": "+6281234567890",
      "company_name": "Toko Budi Sejahtera",
      "has_account": true,
      "created_by": null,
      "addresses": [],
      "created_at": "...",
      "updated_at": "..."
    }
  ],
  "count": 1,
  "offset": 0,
  "limit": 50
}
```

Supported query params: `q` (searches first_name, last_name, email, phone, company_name), `email`, `first_name`, `last_name`, `has_account`, `offset`, `limit`.

```bash
curl -s 'http://localhost:3000/admin/customers?q=budi' | jq '.count'
curl -s 'http://localhost:3000/admin/customers?has_account=true&limit=10' | jq
```

### AC2 — Get customer by ID

```bash
curl -s http://localhost:3000/admin/customers/cus_seed_budi | jq
```

```json
{
  "customer": {
    "id": "cus_seed_budi",
    "first_name": "Budi",
    "last_name": "Santoso",
    "email": "budi@example.com",
    "phone": "+6281234567890",
    "company_name": "Toko Budi Sejahtera",
    "has_account": true,
    "created_by": null,
    "addresses": [],
    "created_at": "...",
    "updated_at": "..."
  }
}
```

Or look up by email first:

```bash
CUS_ID=$(curl -s 'http://localhost:3000/admin/customers?email=budi@example.com' | jq -r '.customers[0].id')
curl -s http://localhost:3000/admin/customers/$CUS_ID | jq
```

### AC3 — List carts (admin)

Admin view of all carts including completed ones (toko-rs extension — Medusa has no admin cart list):

```bash
curl -s http://localhost:3000/admin/carts | jq
```

```json
{
  "carts": [
    {
      "id": "cart_01KQ...",
      "email": "buyer@example.com",
      "currency_code": "idr",
      "completed_at": null,
      "items": [],
      "item_total": 0,
      "total": 0,
      "..."
    }
  ],
  "count": 1,
  "offset": 0,
  "limit": 50
}
```

Supported query params: `id`, `customer_id`, `offset`, `limit`.

```bash
curl -s 'http://localhost:3000/admin/carts?customer_id=cus_seed_budi' | jq
```

### AC4 — Cancel order

Cancel a pending order. Sets order status to `canceled` and payment status to `canceled`. Rejects already-canceled or completed orders (400).

```bash
# Complete a cart to create an order first (see Step 7)
ORDER_ID=$(curl -s -X POST http://localhost:3000/store/carts/$CART_ID/complete | jq -r '.order.id')

curl -s -X POST http://localhost:3000/admin/orders/$ORDER_ID/cancel | jq '.order | {id, status, canceled_at}'
```

```json
{
  "id": "order_01KQ...",
  "status": "canceled",
  "canceled_at": "2026-05-01T..."
}
```

Error cases:

```bash
# Already canceled → 400
curl -s -X POST http://localhost:3000/admin/orders/$ORDER_ID/cancel | jq
# { "code": "invalid_request_error", "type": "invalid_data", "message": "Order is already canceled" }

# Already completed → 400
curl -s -X POST http://localhost:3000/admin/orders/$ORDER_ID/cancel | jq
# { "code": "invalid_request_error", "type": "invalid_data", "message": "Cannot cancel a completed order" }
```

### AC5 — Complete order (admin)

Mark a pending order as completed (toko-rs extension — Medusa has no `POST /admin/orders/:id/complete`). Rejects already-completed or canceled orders (400).

```bash
# Create a fresh order first (see Step 7)
ORDER_ID2=$(curl -s -X POST http://localhost:3000/store/carts/$CART_ID2/complete | jq -r '.order.id')

curl -s -X POST http://localhost:3000/admin/orders/$ORDER_ID2/complete | jq '.order | {id, status}'
```

```json
{
  "id": "order_01KQ...",
  "status": "completed"
}
```

Error cases:

```bash
# Already completed → 400
curl -s -X POST http://localhost:3000/admin/orders/$ORDER_ID2/complete | jq
# { "code": "invalid_request_error", "type": "invalid_data", "message": "Order is already completed" }

# Canceled order → 400
curl -s -X POST http://localhost:3000/admin/orders/$ORDER_ID/cancel >/dev/null
curl -s -X POST http://localhost:3000/admin/orders/$ORDER_ID/complete | jq
# { "code": "invalid_request_error", "type": "invalid_data", "message": "Cannot complete a canceled order" }
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
{ "code": "invalid_request_error", "type": "invalid_data", "message": "Cannot complete an empty cart" }
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
{ "code": "invalid_request_error", "type": "invalid_data", "message": "Cart is already completed" }
```

**Already-completed cart cannot add items:**
```bash
curl -s -X POST http://localhost:3000/store/carts/$CART_ID/line-items \
  -H 'Content-Type: application/json' \
  -d '{"variant_id": "var_seed_kaos_m", "quantity": 1}' | jq
```
```json
{ "code": "invalid_request_error", "type": "invalid_data", "message": "Cart is already completed" }
```

**Cart cannot be completed twice:**
```bash
curl -s -X POST http://localhost:3000/store/carts/$CART_ID/complete | jq
```
```json
{ "code": "invalid_request_error", "type": "invalid_data", "message": "Cart is already completed" }
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

## Admin: Invoice Operations (Task 32)

### AI1: Configure invoice issuer (first time)

Set up company information for invoice generation. Creates or updates the singleton config:

```bash
curl -s -X POST http://localhost:3000/admin/invoice-config \
  -H 'Content-Type: application/json' \
  -d '{
    "company_name": "Toko Sejahtera",
    "company_address": "Jl. Merdeka No. 10, Jakarta Pusat 10110",
    "company_phone": "+6281234567890",
    "company_email": "admin@tokosejahtera.com",
    "company_logo": "https://example.com/logo.png",
    "notes": "Terima kasih atas pembelian Anda. Pembayaran dalam 30 hari."
  }' | jq
```

```json
{
  "invoice_config": {
    "id": "invcfg_01KQ...",
    "company_name": "Toko Sejahtera",
    "company_address": "Jl. Merdeka No. 10, Jakarta Pusat 10110",
    "company_phone": "+6281234567890",
    "company_email": "admin@tokosejahtera.com",
    "company_logo": "https://example.com/logo.png",
    "notes": "Terima kasih atas pembelian Anda. Pembayaran dalam 30 hari.",
    "created_at": "2026-05-01T...",
    "updated_at": "2026-05-01T..."
  }
}
```

### AI2: Get invoice config

```bash
curl -s http://localhost:3000/admin/invoice-config | jq
```

Returns 404 if not configured yet:

```json
{ "code": "invalid_request_error", "type": "not_found", "message": "Invoice config not found" }
```

### AI3: Update invoice config (partial)

```bash
curl -s -X POST http://localhost:3000/admin/invoice-config \
  -H 'Content-Type: application/json' \
  -d '{"company_logo": "https://example.com/logo-v2.png", "notes": "Updated payment terms"}' | jq '.invoice_config | {company_logo, notes}'
```

```json
{
  "company_logo": "https://example.com/logo-v2.png",
  "notes": "Updated payment terms"
}
```

### AI4: Generate invoice for an order

Invoice is generated on-the-fly from order data + company config:

```bash
ORDER_ID="order_01KQ..."   # from Step 7 or Step 9

curl -s http://localhost:3000/admin/orders/$ORDER_ID/invoice | jq
```

```json
{
  "invoice": {
    "invoice_number": "INV-0001",
    "date": "2026-05-01T...",
    "status": "latest",
    "issuer": {
      "company_name": "Toko Sejahtera",
      "company_address": "Jl. Merdeka No. 10, Jakarta Pusat 10110",
      "company_phone": "+6281234567890",
      "company_email": "admin@tokosejahtera.com",
      "company_logo": "https://example.com/logo-v2.png"
    },
    "order": {
      "id": "order_01KQ...",
      "display_id": 1,
      "status": "pending",
      "email": "buyer@example.com",
      "currency_code": "idr",
      "items": [
        {
          "id": "ordli_01KQ...",
          "title": "Kaos Polos",
          "quantity": 3,
          "unit_price": 75000,
          "product_title": "Kaos Polos",
          "variant_sku": "KAOS-P-M"
        }
      ],
      "item_total": 225000,
      "total": 225000
    },
    "notes": "Updated payment terms"
  }
}
```

Returns 404 if no config or no order.

---

## Reference Tables

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

### Customer ID reference for curl

| Name | ID | Use with |
|---|---|---|
| Budi Santoso | `cus_seed_budi` | `X-Customer-Id` header for order endpoints, `customer_id` in cart creation |

### Endpoint summary (38 methods)

| Method | Path | Section |
|---|---|---|
| GET | `/health` | Step 0 |
| GET | `/store/products` | Step 1 |
| GET | `/store/products/{id}` | Step 2 |
| POST | `/store/carts` | Step 3 |
| GET | `/store/carts/{id}` | C1 |
| POST | `/store/carts/{id}` | C2 |
| POST | `/store/carts/{id}/line-items` | Step 4 |
| POST | `/store/carts/{id}/line-items/{line_id}` | Step 6 |
| DELETE | `/store/carts/{id}/line-items/{line_id}` | C3 |
| POST | `/store/carts/{id}/complete` | Step 7 |
| GET | `/store/orders` | Step 10 |
| GET | `/store/orders/{id}` | Step 11 |
| POST | `/store/customers` | Step 8 |
| GET | `/store/customers/me` | CU1 |
| POST | `/store/customers/me` | CU2 |
| POST | `/admin/products` | A1 |
| GET | `/admin/products` | A3 |
| GET | `/admin/products/{id}` | A4 |
| POST | `/admin/products/{id}` | A5 |
| DELETE | `/admin/products/{id}` | A8 |
| POST | `/admin/products/{id}/variants` | A7 |
| GET | `/admin/products/{id}/variants` | (variant list) |
| GET | `/admin/products/{id}/variants/{variant_id}` | (variant get) |
| POST | `/admin/products/{id}/variants/{variant_id}` | (variant update) |
| DELETE | `/admin/products/{id}/variants/{variant_id}` | (variant delete) |
| GET | `/admin/products/{id}/options` | AO1 |
| POST | `/admin/products/{id}/options` | AO3 |
| GET | `/admin/products/{id}/options/{option_id}` | AO2 |
| POST | `/admin/products/{id}/options/{option_id}` | AO4 |
| DELETE | `/admin/products/{id}/options/{option_id}` | AO5 |
| GET | `/admin/customers` | AC1 |
| GET | `/admin/customers/{id}` | AC2 |
| GET | `/admin/carts` | AC3 |
| POST | `/admin/orders/{id}/cancel` | AC4 |
| POST | `/admin/orders/{id}/complete` | AC5 |
| GET | `/admin/invoice-config` | AI2 |
| POST | `/admin/invoice-config` | AI1 |
| GET | `/admin/orders/{id}/invoice` | AI4 |
