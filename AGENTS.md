# AI Agent Development Guide

This guide provides clear rules and patterns for AI agents working on this Rust backend project.

## 📋 Table of Contents

- [Project Architecture](#project-architecture)
- [Directory Structure](#directory-structure)
- [File Organization Rules](#file-organization-rules)
- [Import Conventions](#import-conventions)
- [Workflow: Adding New API Features](#workflow-adding-new-api-features)
- [Code Patterns & Examples](#code-patterns--examples)
- [Common Pitfalls](#common-pitfalls)
- [Quick Reference](#quick-reference)

---

## 🏗️ Project Architecture

This project follows a **clean layered architecture** with strict separation of concerns:

```
┌─────────────────────────────────────────┐
│          HTTP Request/Response          │
└─────────────────┬───────────────────────┘
                  │
         ┌────────▼────────┐
         │    Handlers     │  ← Controllers (HTTP layer only)
         │  (Controller)   │  ← Extract params, call service, wrap response
         └────────┬────────┘
                  │
         ┌────────▼────────┐
         │    Services     │  ← Business Logic
         │ (Business Logic)│  ← All queries, validation, rules
         └────────┬────────┘
                  │
         ┌────────▼────────┐
         │     Models      │  ← Database Entities
         │  (DB Entities)  │  ← Maps to database tables
         └────────┬────────┘
                  │
         ┌────────▼────────┐
         │      DTOs       │  ← API Contracts
         │ (Request/Response)│ ← What clients see
         └─────────────────┘
```

### Layer Responsibilities

| Layer | Responsibility | Returns | Location |
|-------|---------------|---------|----------|
| **Handlers** | HTTP layer only | `Result<ApiSuccess<T>, AppError>` | `src/handlers/` |
| **Services** | Business logic | `Result<DTO, AppError>` | `src/services/` |
| **Models** | Database entities | Database structs | `src/models/` |
| **DTOs** | API contracts | Request/Response structs | `src/dto/` |

---

## 📁 Directory Structure

```
src/
├── config/          # Configuration (DB, Redis, MQTT)
├── dto/             # ⭐ Request/Response DTOs (API contracts)
│   ├── user_dto.rs
│   └── mod.rs
├── handlers/        # ⭐ HTTP Controllers
│   ├── auth_handler.rs
│   ├── user_handler.rs
│   └── mod.rs
├── interceptors/    # Response & Error handling
│   ├── error.rs
│   ├── response.rs
│   └── mod.rs
├── middleware/      # Auth, Logging
│   ├── auth.rs
│   └── mod.rs
├── models/          # ⭐ Database entities
│   ├── user.rs
│   └── mod.rs
├── queue/           # Queue service
├── routes/          # ⭐ URL routing
│   └── mod.rs
├── services/        # ⭐ Business logic
│   ├── user_service.rs
│   ├── redis_service.rs
│   └── mod.rs
├── utils/           # Utilities
└── main.rs
```

---

## 📝 File Organization Rules

### 1. DTOs (`src/dto/`)

**Purpose**: Define all request and response structures for the API.

**Rules**:
- ✅ One file per feature: `{feature}_dto.rs` (e.g., `user_dto.rs`)
- ✅ Include validation using `validator` crate
- ✅ Suffix names with `Request` or `Response`
- ✅ Export all DTOs from `dto/mod.rs`
- ❌ Never include business logic
- ❌ Never include database queries

**Example Structure** (`src/dto/user_dto.rs`):
```rust
use serde::{Deserialize, Serialize};
use validator::Validate;
use chrono::{DateTime, Utc};

/// Response DTO (what API returns)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Request DTO (what API receives)
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 6, message = "Password must be at least 6 characters"))]
    pub password: String,
}
```

**Export Pattern** (`src/dto/mod.rs`):
```rust
pub mod user_dto;

pub use user_dto::{
    CreateUserRequest,
    UpdateUserRequest,
    UserResponse,
    LoginRequest,
    LoginResponse,
};
```

### 2. Models (`src/models/`)

**Purpose**: Database entities that map to tables.

**Rules**:
- ✅ One model per file: `{entity}.rs` (e.g., `user.rs`)
- ✅ Must implement `FromRow` for SQLx
- ✅ Include `to_response()` method to convert to DTO
- ✅ Export from `models/mod.rs`
- ❌ No validation logic (that's in DTOs)
- ❌ No business logic (that's in services)

**Example Structure** (`src/models/user.rs`):
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::dto::UserResponse;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub name: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(email: String, password_hash: String, name: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            email,
            password_hash,
            name,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Convert to response DTO (removes sensitive data)
    pub fn to_response(&self) -> UserResponse {
        UserResponse {
            id: self.id.clone(),
            email: self.email.clone(),
            name: self.name.clone(),
            is_active: self.is_active,
            created_at: self.created_at,
        }
    }
}
```

**Export Pattern** (`src/models/mod.rs`):
```rust
pub mod user;

pub use user::User;
```

### 3. Services (`src/services/`)

**Purpose**: Business logic layer - all queries, validations, and business rules.

**Rules**:
- ✅ One service per feature: `{feature}_service.rs`
- ✅ Contains ALL database queries
- ✅ Returns DTOs, NOT raw models
- ✅ Export from `services/mod.rs`
- ❌ NEVER returns `ApiSuccess` (that's the handler's job)
- ❌ No HTTP-specific logic

**Example Structure** (`src/services/user_service.rs`):
```rust
use sqlx::PgPool;

use crate::dto::{CreateUserRequest, UserResponse};
use crate::interceptors::AppError;
use crate::models::User;
use crate::utils::{hash_password, validate_request};

#[derive(Clone)]
pub struct UserService {
    pool: PgPool,
}

impl UserService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new user
    pub async fn create_user(&self, request: CreateUserRequest) -> Result<UserResponse, AppError> {
        // Validate
        validate_request(&request)?;

        // Check if exists
        let existing = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(&request.email)
            .fetch_optional(&self.pool)
            .await?;

        if existing.is_some() {
            return Err(AppError::Conflict("Email already exists".to_string()));
        }

        // Hash password
        let password_hash = hash_password(&request.password)?;

        // Create user
        let user = User::new(request.email, password_hash, request.name);

        // Insert
        let inserted = sqlx::query_as::<_, User>(
            "INSERT INTO users (...) VALUES (...) RETURNING *"
        )
        .bind(&user.id)
        .bind(&user.email)
        // ... other bindings
        .fetch_one(&self.pool)
        .await?;

        // Return DTO (not raw model!)
        Ok(inserted.to_response())
    }
}
```

**Export Pattern** (`src/services/mod.rs`):
```rust
pub mod user_service;
pub mod redis_service;

pub use user_service::UserService;
pub use redis_service::RedisService;
```

### 4. Handlers (`src/handlers/`)

**Purpose**: HTTP controller layer - handles request/response only.

**Rules**:
- ✅ One handler per feature: `{feature}_handler.rs`
- ✅ Extract params from HTTP request
- ✅ Call service method
- ✅ Wrap result in `ApiSuccess::new(message, data)`
- ✅ Always returns `Result<ApiSuccess<T>, AppError>`
- ❌ NO business logic
- ❌ NO database queries
- ❌ NO validation (service handles it)

**Example Structure** (`src/handlers/user_handler.rs`):
```rust
use axum::{extract::State, Json};
use sqlx::PgPool;

use crate::dto::{CreateUserRequest, UserResponse};
use crate::interceptors::{ApiSuccess, AppError};
use crate::services::UserService;

/// Create a new user
pub async fn create_user(
    State(pool): State<PgPool>,
    Json(request): Json<CreateUserRequest>,
) -> Result<ApiSuccess<UserResponse>, AppError> {
    // 1. Create service
    let user_service = UserService::new(pool);

    // 2. Call service (all logic is there)
    let user = user_service.create_user(request).await?;

    // 3. Wrap in ApiSuccess and return
    Ok(ApiSuccess::new("User created successfully", user))
}
```

**Export Pattern** (`src/handlers/mod.rs`):
```rust
pub mod auth_handler;
pub mod user_handler;

pub use auth_handler::{register, login};
pub use user_handler::{create_user, get_user, update_user};
```

---

## 📦 Import Conventions

### ✅ Good Import Pattern

Always organize imports in this order:

```rust
// 1. Standard library
use std::collections::HashMap;

// 2. External crates
use axum::{extract::State, Json};
use sqlx::PgPool;
use serde::{Deserialize, Serialize};

// 3. Internal modules (grouped by category)
use crate::dto::{CreateUserRequest, UserResponse};
use crate::interceptors::{ApiSuccess, AppError};
use crate::models::User;
use crate::services::UserService;
use crate::utils::validate_request;
```

### ❌ Bad Import Pattern

```rust
// DON'T: Mixed order
use crate::dto::CreateUserRequest;
use axum::Json;
use crate::models::User;
use std::collections::HashMap;

// DON'T: Import from nested modules directly
use crate::dto::user_dto::CreateUserRequest;  // Wrong!
use crate::dto::CreateUserRequest;             // Correct!
```

### Module Re-exports

Always use re-exports from `mod.rs`:

**❌ Wrong**:
```rust
use crate::dto::user_dto::UserResponse;
```

**✅ Correct**:
```rust
use crate::dto::UserResponse;
```

---

## 🔄 Workflow: Adding New API Features

Follow these steps **in order** when adding a new API feature:

### Step 1: Create DTOs

📁 `src/dto/{feature}_dto.rs`

```rust
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductResponse {
    pub id: String,
    pub name: String,
    pub price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateProductRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,

    #[validate(range(min = 0.0))]
    pub price: f64,
}
```

**Then export** in `src/dto/mod.rs`:
```rust
pub mod product_dto;

pub use product_dto::{ProductResponse, CreateProductRequest};
```

### Step 2: Create Model (if needed)

📁 `src/models/product.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::dto::ProductResponse;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub price: f64,
    pub created_at: DateTime<Utc>,
}

impl Product {
    pub fn new(name: String, price: f64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            price,
            created_at: Utc::now(),
        }
    }

    pub fn to_response(&self) -> ProductResponse {
        ProductResponse {
            id: self.id.clone(),
            name: self.name.clone(),
            price: self.price,
        }
    }
}
```

**Then export** in `src/models/mod.rs`:
```rust
pub mod product;

pub use product::Product;
```

### Step 3: Create Service

📁 `src/services/product_service.rs`

```rust
use sqlx::PgPool;

use crate::dto::{CreateProductRequest, ProductResponse};
use crate::interceptors::AppError;
use crate::models::Product;
use crate::utils::validate_request;

#[derive(Clone)]
pub struct ProductService {
    pool: PgPool,
}

impl ProductService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_product(&self, request: CreateProductRequest) -> Result<ProductResponse, AppError> {
        validate_request(&request)?;

        let product = Product::new(request.name, request.price);

        let inserted = sqlx::query_as::<_, Product>(
            "INSERT INTO products (id, name, price, created_at) VALUES ($1, $2, $3, $4) RETURNING *"
        )
        .bind(&product.id)
        .bind(&product.name)
        .bind(product.price)
        .bind(product.created_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(inserted.to_response())
    }
}
```

**Then export** in `src/services/mod.rs`:
```rust
pub mod product_service;

pub use product_service::ProductService;
```

### Step 4: Create Handler

📁 `src/handlers/product_handler.rs`

```rust
use axum::{extract::State, Json};
use sqlx::PgPool;

use crate::dto::{CreateProductRequest, ProductResponse};
use crate::interceptors::{ApiSuccess, AppError};
use crate::services::ProductService;

pub async fn create_product(
    State(pool): State<PgPool>,
    Json(request): Json<CreateProductRequest>,
) -> Result<ApiSuccess<ProductResponse>, AppError> {
    let product_service = ProductService::new(pool);
    let product = product_service.create_product(request).await?;

    Ok(ApiSuccess::new("Product created successfully", product))
}
```

**Then export** in `src/handlers/mod.rs`:
```rust
pub mod product_handler;

pub use product_handler::create_product;
```

### Step 5: Add Routes

📁 `src/routes/mod.rs`

```rust
use crate::handlers::create_product;

let product_routes = Router::new()
    .route("/products", post(create_product));

// Combine all routes under /api prefix
Router::new()
    .nest("/api", Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(product_routes)  // Add here
    )
    .with_state(pool)
```

### Step 6: Create Migration (if needed)

```bash
sqlx migrate add create_products_table
```

Edit the migration file:
```sql
CREATE TABLE products (
    id VARCHAR(255) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    price DECIMAL(10, 2) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
```

Run migration:
```bash
sqlx migrate run
```

---

## 💡 Code Patterns & Examples

### Pattern 1: Service Method Signature

**✅ Correct**:
```rust
pub async fn create_user(&self, request: CreateUserRequest) -> Result<UserResponse, AppError>
```

**❌ Wrong**:
```rust
// Don't return ApiSuccess from service!
pub async fn create_user(&self, request: CreateUserRequest) -> Result<ApiSuccess<UserResponse>, AppError>
```

### Pattern 2: Handler Method Signature

**✅ Correct**:
```rust
pub async fn create_user(
    State(pool): State<PgPool>,
    Json(request): Json<CreateUserRequest>,
) -> Result<ApiSuccess<UserResponse>, AppError>
```

### Pattern 3: Error Handling

**✅ Correct**:
```rust
// Service
if existing_user.is_some() {
    return Err(AppError::Conflict("Email already exists".to_string()));
}

// Handler (just propagate errors with ?)
let user = user_service.create_user(request).await?;
```

### Pattern 4: Validation

**✅ Correct** (in service):
```rust
pub async fn create_user(&self, request: CreateUserRequest) -> Result<UserResponse, AppError> {
    // Validate first
    validate_request(&request)?;

    // Then business logic
    // ...
}
```

**❌ Wrong** (in handler):
```rust
pub async fn create_user(...) -> Result<ApiSuccess<UserResponse>, AppError> {
    validate_request(&request)?;  // Don't do this in handler!
    // ...
}
```

### Pattern 5: Returning Data

**Service**:
```rust
// Return DTO directly
Ok(user.to_response())
```

**Handler**:
```rust
// Wrap in ApiSuccess
Ok(ApiSuccess::new("User created successfully", user))
```

---

## ⚠️ Common Pitfalls

### ❌ Pitfall 1: Business Logic in Handlers

**Wrong**:
```rust
pub async fn create_user(State(pool): State<PgPool>, ...) -> ... {
    // DON'T do validation, queries, or logic here!
    validate_request(&request)?;
    let user = sqlx::query_as::<_, User>("SELECT...").fetch_one(&pool).await?;
}
```

**Correct**:
```rust
pub async fn create_user(State(pool): State<PgPool>, ...) -> ... {
    let user_service = UserService::new(pool);
    let user = user_service.create_user(request).await?;
    Ok(ApiSuccess::new("Success", user))
}
```

### ❌ Pitfall 2: Returning Raw Models

**Wrong**:
```rust
// Service
pub async fn get_user(&self, id: &str) -> Result<User, AppError> {
    let user = sqlx::query_as::<_, User>("...").fetch_one(&self.pool).await?;
    Ok(user)  // Don't return raw model!
}
```

**Correct**:
```rust
// Service
pub async fn get_user(&self, id: &str) -> Result<UserResponse, AppError> {
    let user = sqlx::query_as::<_, User>("...").fetch_one(&self.pool).await?;
    Ok(user.to_response())  // Return DTO!
}
```

### ❌ Pitfall 3: ApiSuccess in Service

**Wrong**:
```rust
// Service
pub async fn create_user(&self, ...) -> Result<ApiSuccess<UserResponse>, AppError> {
    // ...
    Ok(ApiSuccess::new("Success", user))  // Don't do this!
}
```

**Correct**:
```rust
// Service
pub async fn create_user(&self, ...) -> Result<UserResponse, AppError> {
    // ...
    Ok(user)  // Just return the DTO
}
```

### ❌ Pitfall 4: Direct Nested Imports

**Wrong**:
```rust
use crate::dto::user_dto::UserResponse;
use crate::models::user::User;
```

**Correct**:
```rust
use crate::dto::UserResponse;
use crate::models::User;
```

---

## 📖 Quick Reference

### File Creation Checklist

When adding a new feature (e.g., "Product"):

- [ ] Create `src/dto/product_dto.rs`
- [ ] Export DTOs in `src/dto/mod.rs`
- [ ] Create `src/models/product.rs` (if needed)
- [ ] Export model in `src/models/mod.rs`
- [ ] Create `src/services/product_service.rs`
- [ ] Export service in `src/services/mod.rs`
- [ ] Create `src/handlers/product_handler.rs`
- [ ] Export handlers in `src/handlers/mod.rs`
- [ ] Add routes in `src/routes/mod.rs`
- [ ] Create migration if needed

### Layer Responsibilities Cheat Sheet

```
┌─────────────────────────────────────────────────┐
│ HANDLER                                         │
│ - Extract HTTP params                           │
│ - Call service                                  │
│ - Wrap in ApiSuccess::new(msg, data)            │
│ - Return Result<ApiSuccess<T>, AppError>        │
└─────────────────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────┐
│ SERVICE                                         │
│ - validate_request(&dto)?                       │
│ - All database queries                          │
│ - All business logic                            │
│ - Return Result<DTO, AppError>                  │
└─────────────────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────┐
│ MODEL                                           │
│ - Database entity struct                        │
│ - FromRow implementation                        │
│ - to_response() → DTO                           │
└─────────────────────────────────────────────────┘
```

### Return Type Guide

| Layer | Method Returns |
|-------|---------------|
| Handler | `Result<ApiSuccess<DTO>, AppError>` |
| Service | `Result<DTO, AppError>` |
| Model | `DTO` (via `to_response()`) |

---

## 🎯 Summary

**Golden Rules**:
1. **Handlers** = Controllers (HTTP only, no logic)
2. **Services** = Business logic (queries, validations, rules)
3. **Models** = Database entities (one-to-one with tables)
4. **DTOs** = API contracts (what clients see)
5. Services return **DTOs**, not raw models
6. Services **NEVER** return `ApiSuccess`
7. Handlers **ALWAYS** wrap service results in `ApiSuccess`
8. Use module re-exports, never direct nested imports

Follow these rules, and your code will be clean, testable, and maintainable! 🦀
