# Toko-rs Implementation Checklist

### Phase 0: Scaffold & Setup
- [x] 0.1 Initialize Git & core Cargo workspace (Edition 2021)
- [x] 0.2 Add Medusa source as vendor submodule
- [x] 0.3 Finalize dependencies (axum, sqlx, tokio) and MSRV 1.85
- [x] 0.4 Write foundation (config, db connection pool, error types) + routing skeleton
- [x] 0.5 Write raw SQL migrations
- [x] 0.6 Generate boilerplate Makefile

### Phase 1-A: Product Module (Spec-driven TDD)
- [x] **1A.1 Define Models & Types**: Map OpenAPI spec to `ProductWithRelations`
- [x] **1A.2 Setup Integration Testing**: API Contract testing.
- [x] **1A.3 Define Repository Framework**: `DatabaseRepo` polymorphism.
- [x] **1A.4 Implement Product Creation**: Atomic database bindings.
- [x] **1A.5 Pass Tests**: Passed with >92% Route Coverage!

### Phase 1-B: Cart Module
- [ ] **1B.1 Define Models & Specs**: Implement `src/cart/models.rs` and `src/cart/types.rs` mapped to schemas.
- [ ] **1B.2 Write Tests**: Implement spec-driven testing in `tests/cart_test.rs`.
- [ ] **1B.3 Setup Repository Flow**: Wire up `CartRepository` into `AppDb` Enum dispatch.
- [ ] **1B.4 Implement Create & Update**: Route stubs and functional inserts.
- [ ] **1B.5 Implement Line Item Management**: Bind Variants safely (no inventory decrementing until checkout).

### Phase 1-C: Order Module
- [ ] 1C.1 Define Models & Specs
- [ ] 1C.2 Write Tests
- [ ] 1C.3 Implement Order Generation

### Phase 1-D: Customer Module
- [ ] 1D.1 Define Models & Specs
- [ ] 1D.2 Write Tests
- [ ] 1D.3 Implement Registration & Profile
