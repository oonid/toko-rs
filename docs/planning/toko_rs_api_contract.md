# toko-rs — P1 API Contract

> **Source**: Extracted from Medusa's actual `route.ts`, `validators.ts`, and `query-config.ts` files.
> **Convention**: All responses use Medusa's **root wrapper** pattern: `{"product": {...}}`, `{"cart": {...}}`, `{"products": [...], "count", "offset", "limit"}`.

---

## Endpoint Summary — P1 (20 endpoints)

### Admin API (6 endpoints — product management)

| Method | Path | Purpose |
|---|---|---|
| POST | `/admin/products` | Create product with options + variants |
| POST | `/admin/products/:id` | Update product |
| DELETE | `/admin/products/:id` | Soft delete product |
| GET | `/admin/products` | List products |
| GET | `/admin/products/:id` | Get single product |
| POST | `/admin/products/:id/variants` | Add variant to product |

### Store API (14 endpoints — customer-facing)

| Method | Path | Purpose |
|---|---|---|
| GET | `/store/products` | List published products |
| GET | `/store/products/:id` | Get single product |
| POST | `/store/carts` | Create a new cart |
| GET | `/store/carts/:id` | Get cart with items |
| POST | `/store/carts/:id` | Update cart (email, metadata) |
| POST | `/store/carts/:id/line-items` | Add item to cart |
| POST | `/store/carts/:id/line-items/:line_id` | Update line item quantity |
| DELETE | `/store/carts/:id/line-items/:line_id` | Remove line item |
| POST | `/store/carts/:id/complete` | Complete cart → Order |
| GET | `/store/orders` | List customer's orders |
| GET | `/store/orders/:id` | Get order detail |
| POST | `/store/customers` | Register customer |
| GET | `/store/customers/me` | Get profile |
| POST | `/store/customers/me` | Update profile |

---

## Admin API Contracts

> Source: `packages/medusa/src/api/admin/products/`

### `POST /admin/products` — Create Product

**Request body** (from `validators.ts` → `CreateProduct` schema):

```json
{
  "title": "Classic T-Shirt",           // required
  "handle": "classic-t-shirt",          // optional, auto-generated from title via slugify
  "description": "A comfortable tee",   // optional
  "status": "draft",                    // optional, default: "draft"
  "thumbnail": "https://...",           // optional
  "metadata": {},                       // optional, arbitrary JSON
  "options": [                          // optional
    {
      "title": "Size",
      "values": ["S", "M", "L"]
    },
    {
      "title": "Color",
      "values": ["Red", "Blue"]
    }
  ],
  "variants": [                         // optional
    {
      "title": "Small Red",
      "sku": "TS-S-RED",
      "prices": [
        {"currency_code": "usd", "amount": 2500}
      ],
      "options": {"Size": "S", "Color": "Red"},
      "metadata": {}
    }
  ]
}
```

**toko-rs MVP simplification**:
- `prices` array → single `price` integer (cents) since we skip the Pricing module
- Skip: `images`, `is_giftcard`, `discountable`, `type_id`, `collection_id`, `categories`, `tags`, `sales_channels`, `shipping_profile_id`, physical attributes

**Simplified request**:

```json
{
  "title": "Classic T-Shirt",
  "handle": "classic-t-shirt",
  "description": "A comfortable tee",
  "status": "draft",
  "thumbnail": null,
  "metadata": null,
  "options": [
    {"title": "Size", "values": ["S", "M", "L"]}
  ],
  "variants": [
    {
      "title": "Small",
      "sku": "TS-S",
      "price": 2500,
      "options": {"Size": "S"},
      "metadata": null
    }
  ]
}
```

**Response** (`200`):

```json
{
  "product": {
    "id": "prod_01JX...",
    "title": "Classic T-Shirt",
    "handle": "classic-t-shirt",
    "description": "A comfortable tee",
    "status": "draft",
    "thumbnail": null,
    "metadata": null,
    "options": [
      {
        "id": "opt_01JX...",
        "title": "Size",
        "product_id": "prod_01JX...",
        "values": [
          {"id": "optval_01JX...", "value": "S"},
          {"id": "optval_02JX...", "value": "M"},
          {"id": "optval_03JX...", "value": "L"}
        ],
        "created_at": "2026-04-06T00:00:00Z",
        "updated_at": "2026-04-06T00:00:00Z"
      }
    ],
    "variants": [
      {
        "id": "variant_01JX...",
        "title": "Small",
        "sku": "TS-S",
        "price": 2500,
        "product_id": "prod_01JX...",
        "variant_rank": 0,
        "options": [
          {"id": "optval_01JX...", "value": "S", "option_id": "opt_01JX..."}
        ],
        "metadata": null,
        "created_at": "2026-04-06T00:00:00Z",
        "updated_at": "2026-04-06T00:00:00Z"
      }
    ],
    "created_at": "2026-04-06T00:00:00Z",
    "updated_at": "2026-04-06T00:00:00Z"
  }
}
```

---

### `POST /admin/products/:id` — Update Product

**Request body** (from `validators.ts` → `UpdateProduct` schema — all fields optional):

```json
{
  "title": "Updated T-Shirt",
  "description": "Updated description",
  "status": "published",
  "metadata": {"featured": true}
}
```

**Response** (`200`): Same shape as create — `{"product": {...}}`

---

### `DELETE /admin/products/:id` — Soft Delete

**Request body**: None

**Response** (`200`):

```json
{
  "id": "prod_01JX...",
  "object": "product",
  "deleted": true
}
```

---

### `GET /admin/products` — List Products

**Query params**: `?offset=0&limit=50&order=-created_at&status[]=draft&status[]=published`

**Response** (`200`):

```json
{
  "products": [
    { "id": "prod_01JX...", "title": "...", "...": "..." }
  ],
  "count": 42,
  "offset": 0,
  "limit": 50
}
```

---

### `GET /admin/products/:id` — Get Product

**Response** (`200`): `{"product": {...}}` — same shape as create response

---

### `POST /admin/products/:id/variants` — Add Variant

**Request body**:

```json
{
  "title": "Extra Large Blue",
  "sku": "TS-XL-BLUE",
  "price": 2900,
  "options": {"Size": "XL", "Color": "Blue"},
  "metadata": null
}
```

**Response** (`200`): Returns full product — `{"product": {...}}`

---

## Store API Contracts

> Source: `packages/medusa/src/api/store/*/`

### `GET /store/products` — List Published Products

**Query params**: `?offset=0&limit=50`

> [!NOTE]
> Store API only returns products with `status = "published"`. This filter is implicit, not a query param.

**Response** (`200`):

```json
{
  "products": [
    {
      "id": "prod_01JX...",
      "title": "Classic T-Shirt",
      "handle": "classic-t-shirt",
      "description": "A comfortable tee",
      "status": "published",
      "thumbnail": null,
      "metadata": null,
      "options": [
        {
          "id": "opt_01JX...",
          "title": "Size",
          "product_id": "prod_01JX...",
          "values": [
            {"id": "optval_01JX...", "value": "S"},
            {"id": "optval_02JX...", "value": "M"}
          ]
        }
      ],
      "variants": [
        {
          "id": "variant_01JX...",
          "title": "Small",
          "sku": "TS-S",
          "price": 2500,
          "variant_rank": 0,
          "options": [
            {"id": "optval_01JX...", "value": "S"}
          ]
        }
      ],
      "created_at": "2026-04-06T00:00:00Z",
      "updated_at": "2026-04-06T00:00:00Z"
    }
  ],
  "count": 1,
  "offset": 0,
  "limit": 50
}
```

---

### `GET /store/products/:id` — Get Single Product

**Response** (`200`): `{"product": {...}}` — same object shape as list item

---

### `POST /store/carts` — Create Cart

**Request body** (from `validators.ts` → `CreateCart`):

```json
{
  "currency_code": "usd",
  "email": "buyer@example.com",
  "metadata": null
}
```

**toko-rs MVP**: Skip `region_id`, `sales_channel_id`, `shipping_address`, `billing_address`, `items`, `promo_codes`, `locale`

**Response** (`200`):

```json
{
  "cart": {
    "id": "cart_01JX...",
    "customer_id": null,
    "email": "buyer@example.com",
    "currency_code": "usd",
    "shipping_address": null,
    "billing_address": null,
    "metadata": null,
    "items": [],
    "item_total": 0,
    "total": 0,
    "completed_at": null,
    "created_at": "2026-04-06T00:00:00Z",
    "updated_at": "2026-04-06T00:00:00Z"
  }
}
```

---

### `GET /store/carts/:id` — Get Cart

**Response** (`200`): `{"cart": {...}}` — same shape as create, with `items` populated

```json
{
  "cart": {
    "id": "cart_01JX...",
    "email": "buyer@example.com",
    "currency_code": "usd",
    "items": [
      {
        "id": "cali_01JX...",
        "title": "Classic T-Shirt",
        "quantity": 2,
        "unit_price": 2500,
        "variant_id": "variant_01JX...",
        "product_id": "prod_01JX...",
        "snapshot": {
          "product_title": "Classic T-Shirt",
          "variant_title": "Small",
          "variant_sku": "TS-S"
        },
        "metadata": null,
        "total": 5000,
        "created_at": "2026-04-06T00:00:00Z",
        "updated_at": "2026-04-06T00:00:00Z"
      }
    ],
    "item_total": 5000,
    "total": 5000,
    "completed_at": null,
    "created_at": "2026-04-06T00:00:00Z",
    "updated_at": "2026-04-06T00:00:00Z"
  }
}
```

---

### `POST /store/carts/:id` — Update Cart

**Request body** (from `validators.ts` → `UpdateCart`):

```json
{
  "email": "new@example.com",
  "metadata": {"note": "gift wrapping please"}
}
```

**Response** (`200`): `{"cart": {...}}`

---

### `POST /store/carts/:id/line-items` — Add Line Item

**Request body** (from `validators.ts` → `StoreAddCartLineItem`):

```json
{
  "variant_id": "variant_01JX...",
  "quantity": 2,
  "metadata": null
}
```

> [!IMPORTANT]
> Medusa requires `quantity > 0` (validated with `z.number().gt(0)`).

**Response** (`200`): `{"cart": {...}}` — returns full updated cart

---

### `POST /store/carts/:id/line-items/:line_id` — Update Line Item

**Request body** (from `validators.ts` → `StoreUpdateCartLineItem`):

```json
{
  "quantity": 3,
  "metadata": null
}
```

> [!NOTE]
> Medusa allows `quantity >= 0` (`z.number().gte(0)`). Setting `quantity: 0` effectively removes the item.

**Response** (`200`): `{"cart": {...}}`

---

### `DELETE /store/carts/:id/line-items/:line_id` — Remove Line Item

**Request body**: None

**Response** (`200`): `{"cart": {...}}` — returns updated cart without the item

---

### `POST /store/carts/:id/complete` — Complete Cart

**Request body**: None

**Response** (`200`):

```json
{
  "type": "order",
  "order": {
    "id": "order_01JX...",
    "display_id": 1,
    "customer_id": "cus_01JX...",
    "email": "buyer@example.com",
    "currency_code": "usd",
    "status": "pending",
    "shipping_address": null,
    "billing_address": null,
    "metadata": null,
    "items": [
      {
        "id": "oli_01JX...",
        "title": "Classic T-Shirt",
        "quantity": 2,
        "unit_price": 2500,
        "variant_id": "variant_01JX...",
        "product_id": "prod_01JX...",
        "snapshot": {
          "product_title": "Classic T-Shirt",
          "variant_title": "Small",
          "variant_sku": "TS-S"
        },
        "total": 5000
      }
    ],
    "item_total": 5000,
    "total": 5000,
    "payment": {
      "id": "pay_01JX...",
      "status": "pending",
      "amount": 5000,
      "currency_code": "usd",
      "provider": "manual"
    },
    "created_at": "2026-04-06T00:00:00Z",
    "updated_at": "2026-04-06T00:00:00Z"
  }
}
```

---

### `GET /store/orders` — List Orders

**Query params**: `?offset=0&limit=20`

> Requires customer context (header `X-Customer-Id` or auth token in P2)

**Response** (`200`):

```json
{
  "orders": [
    {
      "id": "order_01JX...",
      "display_id": 1,
      "status": "pending",
      "currency_code": "usd",
      "email": "buyer@example.com",
      "item_total": 5000,
      "total": 5000,
      "items": [...],
      "created_at": "2026-04-06T00:00:00Z"
    }
  ],
  "count": 1,
  "offset": 0,
  "limit": 20
}
```

---

### `GET /store/orders/:id` — Get Order Detail

**Response** (`200`): `{"order": {...}}` — full order with items and payment

---

### `POST /store/customers` — Register

**Request body**:

```json
{
  "first_name": "Budi",
  "last_name": "Santoso",
  "email": "budi@example.com",
  "phone": "+6281234567890"
}
```

**Response** (`200`):

```json
{
  "customer": {
    "id": "cus_01JX...",
    "first_name": "Budi",
    "last_name": "Santoso",
    "email": "budi@example.com",
    "phone": "+6281234567890",
    "has_account": true,
    "metadata": null,
    "created_at": "2026-04-06T00:00:00Z",
    "updated_at": "2026-04-06T00:00:00Z"
  }
}
```

---

### `GET /store/customers/me` — Get Profile

**Headers**: `X-Customer-Id: cus_01JX...` (P1 stub for auth)

**Response** (`200`): `{"customer": {...}}`

---

### `POST /store/customers/me` — Update Profile

**Request body** (all optional):

```json
{
  "first_name": "Budi",
  "phone": "+6289876543210",
  "metadata": {"preferred_lang": "id"}
}
```

**Response** (`200`): `{"customer": {...}}`

---

## Error Responses

All errors follow `MedusaError` shape (from `errors.ts`):

```json
{
  "type": "not_found",
  "message": "Product with id prod_01JX... was not found"
}
```

| HTTP Status | Error Type | When |
|---|---|---|
| 400 | `invalid_data` | Missing required field, validation failure |
| 404 | `not_found` | Entity doesn't exist |
| 409 | `duplicate_error` | Unique constraint (e.g., duplicate handle/SKU) |
| 409 | `unexpected_state` | Cart already completed, invalid state transition |
| 500 | `database_error` | Internal DB error (message sanitized) |

---

## Response Pattern Summary

| Pattern | Usage | Medusa Source |
|---|---|---|
| `{"product": {...}}` | Single entity response | `res.json({ product: ... })` |
| `{"products": [...], "count", "offset", "limit"}` | List response | `res.json({ products, count, offset, limit })` |
| `{"cart": {...}}` | Cart mutations return full cart | `res.json({ cart })` |
| `{"type": "order", "order": {...}}` | Cart complete response | Medusa complete workflow |
| `{"id", "object", "deleted": true}` | Delete response | Medusa delete pattern |
| `{"type": "...", "message": "..."}` | Error response | `MedusaError` |

> [!TIP]
> **Key convention**: Every mutation endpoint (POST, DELETE) against cart returns the **full updated cart**. This eliminates the need for the client to re-fetch after every action — aligns with Medusa's pattern and is ideal for a chat-based system where each turn needs the latest state.
