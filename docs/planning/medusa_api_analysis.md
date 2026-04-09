# Medusa v2 OpenAPI Spec Analysis — Rust MVP Mapping

> **Source**: `medusajs/medusa` @ `develop` branch
> **Spec Path**: `www/utils/generated/oas-output/`
> **Spec Version**: OpenAPI 3.0.0, Medusa API v2.0.0

---

## Store API — Full Endpoint Inventory

The Store API is the customer-facing surface — this is what your chat system will call.

### 🟢 Priority 1 — Core MVP (Products → Cart → Checkout)

These are the **minimum viable flow** for a chat-based purchase.

#### Products (read-only catalog browsing)
| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/store/products` | List products (with filters, pagination) |
| `GET` | `/store/products/:id` | Get a single product (with variants, options, prices) |

#### Carts (the heart of the transaction)
| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/store/carts` | Create a new cart |
| `GET` | `/store/carts/:id` | Get cart by ID |
| `POST` | `/store/carts/:id` | Update cart (email, addresses, region, etc.) |
| `POST` | `/store/carts/:id/line-items` | Add a line item (variant + quantity) |
| `POST` | `/store/carts/:id/line-items/:line_id` | Update a line item (change quantity) |
| `DELETE` | `/store/carts/:id/line-items/:line_id` | Remove a line item |
| `POST` | `/store/carts/:id/complete` | Complete cart → creates an Order |

#### Orders (view placed orders)
| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/store/orders` | List customer's orders |
| `GET` | `/store/orders/:id` | Get order details |

**Endpoint Count: 10** — This is your MVP.

---

### 🟡 Priority 2 — Essential for Real Checkout

These complete the checkout flow with payments and shipping.

#### Cart — Shipping & Payments
| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/store/carts/:id/shipping-methods` | Set shipping method on cart |
| `POST` | `/store/carts/:id/taxes` | Calculate taxes for cart |
| `POST` | `/store/carts/:id/customer` | Transfer/set customer on cart |

#### Payment Collections
| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/store/payment-collections` | Create a payment collection |
| `POST` | `/store/payment-collections/:id/payment-sessions` | Initialize a payment session |

#### Payment Providers
| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/store/payment-providers` | List available payment providers |

#### Shipping Options
| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/store/shipping-options` | List available shipping options |
| `POST` | `/store/shipping-options/:id/calculate` | Calculate shipping price |

**Endpoint Count: 8**

---

### 🟡 Priority 2 — Customer Management

#### Customers
| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/store/customers` | Register a new customer |
| `GET` | `/store/customers/me` | Get logged-in customer profile |
| `POST` | `/store/customers/me` | Update customer profile |
| `GET` | `/store/customers/me/addresses` | List customer addresses |
| `GET` | `/store/customers/me/addresses/:address_id` | Get specific address |
| `POST` | `/store/customers/me/addresses` | Add a new address |
| `POST` | `/store/customers/me/addresses/:address_id` | Update an address |
| `DELETE` | `/store/customers/me/addresses/:address_id` | Delete an address |

#### Auth
| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/auth/:actor_type/:auth_provider` | Authenticate (login) |
| `POST` | `/auth/:actor_type/:auth_provider/register` | Register auth identity |
| `POST` | `/auth/:actor_type/:auth_provider/callback` | Auth callback (OAuth) |
| `POST` | `/auth/:actor_type/:auth_provider/update` | Update auth credentials |
| `POST` | `/auth/:actor_type/:auth_provider/reset-password` | Reset password |
| `POST` | `/auth/session` | Create session |
| `DELETE` | `/auth/session` | Delete session (logout) |
| `POST` | `/auth/token/refresh` | Refresh JWT token |

**Endpoint Count: 16**

---

### 🔵 Priority 3 — Catalog Organization

#### Product Categories
| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/store/product-categories` | List categories (nested, tree) |
| `GET` | `/store/product-categories/:id` | Get single category |

#### Collections
| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/store/collections` | List product collections |
| `GET` | `/store/collections/:id` | Get single collection |

#### Product Tags
| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/store/product-tags` | List product tags |
| `GET` | `/store/product-tags/:id` | Get single tag |

#### Product Types
| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/store/product-types` | List product types |
| `GET` | `/store/product-types/:id` | Get single type |

**Endpoint Count: 8**

---

### 🔵 Priority 3 — Region & Currency

#### Regions
| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/store/regions` | List available regions |
| `GET` | `/store/regions/:id` | Get specific region |

#### Currencies
| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/store/currencies` | List available currencies |
| `GET` | `/store/currencies/:code` | Get single currency |

**Endpoint Count: 4**

---

### ⚪ Priority 4 — Post-Purchase & Extras

#### Returns
| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/store/returns` | Create a return request |
| `GET` | `/store/return-reasons` | List return reasons |
| `GET` | `/store/return-reasons/:id` | Get return reason |

#### Order Transfers
| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/store/orders/:id/transfer/request` | Request order transfer |
| `POST` | `/store/orders/:id/transfer/accept` | Accept transfer |
| `POST` | `/store/orders/:id/transfer/decline` | Decline transfer |
| `POST` | `/store/orders/:id/transfer/cancel` | Cancel transfer |

#### Cart — Promotions & Gift Cards
| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/store/carts/:id/promotions` | Apply promotion code |
| `DELETE` | `/store/carts/:id/promotions` | Remove promotion |
| `POST` | `/store/carts/:id/gift-cards` | Apply gift card |
| `DELETE` | `/store/carts/:id/gift-cards` | Remove gift card |
| `POST` | `/store/carts/:id/store-credits` | Apply store credit |

#### Gift Cards (Cloud only)
| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/store/gift-cards/:idOrCode` | Get gift card |

#### Store Credit Accounts (Cloud only)
| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/store/store-credit-accounts` | List store credit accounts |
| `GET` | `/store/store-credit-accounts/:id` | Get store credit account |

#### Locales
| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/store/locales` | List available locales |

**Endpoint Count: 16**

---

## Admin API — Module Overview

The Admin API is massive (~40+ modules). For the MVP, you only need a subset to seed/manage data.

| Module | Description | MVP Need? |
|--------|-------------|-----------|
| **Products** | CRUD products, variants, options, images | ✅ Essential — to seed catalog |
| **Product Categories** | Manage category tree | 🔵 Later |
| **Product Tags** | Manage tags | 🔵 Later |
| **Product Types** | Manage types | 🔵 Later |
| **Collections** | Manage collections | 🔵 Later |
| **Product Variants** | Manage variants | ✅ Part of Products |
| **Inventory Items** | Manage stock levels | 🟡 After MVP |
| **Customers** | Manage customers | 🟡 After MVP |
| **Customer Groups** | Group customers | ⚪ Later |
| **Orders** | View/manage orders | ✅ Essential — to process orders |
| **Order Edits** | Edit orders | ⚪ Later |
| **Order Changes** | Track order changes | ⚪ Later |
| **Draft Orders** | Create orders manually | ⚪ Later |
| **Claims** | Handle defective items | ⚪ Later |
| **Exchanges** | Handle item exchanges | ⚪ Later |
| **Returns** | Handle returns | ⚪ Later |
| **Refund Reasons** | Reasons for refunds | ⚪ Later |
| **Payment Collections** | Manage payments | 🟡 After MVP |
| **Payments** | Capture/refund payments | 🟡 After MVP |
| **Promotions** | Manage discounts/promos | ⚪ Later |
| **Campaigns** | Group promotions | ⚪ Later |
| **Price Lists** | Special pricing | ⚪ Later |
| **Price Preferences** | Tax-inclusive pricing | ⚪ Later |
| **Regions** | Manage regions | 🟡 After MVP |
| **Currencies** | Manage currencies | 🟡 After MVP |
| **Sales Channels** | Multi-channel sales | ⚪ Later |
| **Stores** | Store settings | 🟡 After MVP |
| **Shipping Options/Profiles** | Manage fulfillment | 🟡 After MVP |
| **Fulfillment Sets/Providers** | Fulfillment config | ⚪ Later |
| **Fulfillments** | Track shipments | ⚪ Later |
| **Stock Locations** | Warehouse locations | ⚪ Later |
| **Reservations** | Reserve inventory | ⚪ Later |
| **Tax Rates/Regions/Providers** | Tax configuration | ⚪ Later |
| **Users/Invites** | Admin users | ⚪ Later |
| **API Keys** | Manage API keys | ⚪ Later |
| **Notifications** | System notifications | ⚪ Later |
| **Uploads** | File uploads | ⚪ Later |
| **Workflow Executions** | Track workflows | ⚪ Later |
| **Gift Cards** | Gift card management | ⚪ Later (Cloud) |
| **Store Credit Accounts** | Store credits | ⚪ Later (Cloud) |
| **Translations/Locales** | i18n | ⚪ Later |

---

## Proposed Workspace Structure

```
medusa-rs/
├── Cargo.toml                     # [workspace] root
├── README.md
│
├── specs/                         # OpenAPI specs (reference)
│   ├── store.oas.base.yaml        # ← downloaded from Medusa repo
│   ├── admin.oas.base.yaml
│   └── operations/
│       ├── store/                  # Individual store operation specs
│       └── admin/                  # Individual admin operation specs
│
├── vendor/                        # Git submodules for reference
│   └── medusa/                    # ← git submodule: github.com/medusajs/medusa
│       └── ...                    # Full medusa-core for implementation reference
│
├── crates/
│   ├── core/                      # Shared foundation
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── db.rs              # Database pool abstraction (SQLite/PG)
│   │       ├── error.rs           # Error types (matches Medusa Error schema)
│   │       ├── types.rs           # Common types (ID, timestamps, pagination)
│   │       └── pagination.rs      # Pagination request/response
│   │
│   ├── product/                   # Product Module
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── model.rs           # Product, Variant, Option, Price models
│   │       ├── repository.rs      # DB queries
│   │       ├── service.rs         # Business logic
│   │       └── routes.rs          # Axum handlers (GET /store/products, etc.)
│   │
│   ├── cart/                      # Cart Module
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── model.rs           # Cart, LineItem, ShippingMethod
│   │       ├── repository.rs
│   │       ├── service.rs         # Add/remove items, calculate totals
│   │       └── routes.rs
│   │
│   ├── order/                     # Order Module
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── model.rs           # Order, OrderLineItem, OrderStatus
│   │       ├── repository.rs
│   │       ├── service.rs         # Cart→Order conversion, status tracking
│   │       └── routes.rs
│   │
│   ├── customer/                  # Customer Module
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── model.rs           # Customer, Address
│   │       ├── repository.rs
│   │       ├── service.rs
│   │       └── routes.rs
│   │
│   ├── payment/                   # Payment Module (simplified)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── model.rs           # PaymentCollection, PaymentSession
│   │       ├── repository.rs
│   │       ├── service.rs
│   │       └── routes.rs
│   │
│   └── api/                       # API composition crate (the binary)
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs            # Axum server, composes all module routers
│           ├── config.rs          # App configuration
│           ├── middleware.rs       # Auth middleware, error handling
│           └── state.rs           # Shared application state
│
├── migrations/                    # SQLx migrations (SQLite)
│   ├── 001_create_products.sql
│   ├── 002_create_carts.sql
│   ├── 003_create_orders.sql
│   ├── 004_create_customers.sql
│   └── 005_create_payments.sql
│
└── tests/                         # Integration tests (TDD contracts)
    ├── common/
    │   └── mod.rs                 # Test helpers, server setup
    ├── store_products_test.rs
    ├── store_carts_test.rs
    ├── store_orders_test.rs
    └── store_customers_test.rs
```

---

## Summary Statistics

| Category | Endpoints |
|----------|-----------|
| Store API — Priority 1 (MVP) | **10** |
| Store API — Priority 2 (Checkout + Customer) | **24** |
| Store API — Priority 3 (Catalog org + Region) | **12** |
| Store API — Priority 4 (Post-purchase, promos) | **16** |
| **Total Store API** | **62** |
| Admin API Modules | **~40** |

## Spec-Driven TDD Workflow

```
1. Pick a module (e.g., Product)
2. Download operation specs from:
   github.com/medusajs/medusa/.../operations/store/get_store_products.ts
3. Extract request params + response schema
4. Write Rust integration test matching the contract
5. Implement handler → service → repository
6. Test passes → move to next endpoint
```

> [!TIP]
> The individual operation `.ts` files in `operations/store/` contain **full request/response schemas** including query parameters, path parameters, and response body shapes. These are your TDD contracts.

> [!IMPORTANT]
> The specs reference schemas in a separate `schemas/` directory. You'll want to download the relevant schema files too (e.g., `StoreProduct`, `StoreCart`, `StoreOrder`) to get the complete data model definitions.
