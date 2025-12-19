# Rust Backend Template

A professional, production-ready Rust backend template with comprehensive features including JWT authentication, Redis queue service, MQTT support, and more.

## Features

- **Web Framework**: Axum with async/await support
- **Database**: PostgreSQL with SQLx and migrations
- **Caching**: Redis with connection pooling
- **Queue System**: Custom Redis-based queue service (similar to BeeQueue)
- **Authentication**: JWT-based authentication middleware
- **MQTT**: Full MQTT client support for IoT applications
- **Response Interceptors**: Standardized API response format
- **Error Handling**: Comprehensive error handling with custom error types
- **Validation**: Request validation using validator
- **Logging**: Structured logging with tracing (console + file)
- **Security**: Password hashing with bcrypt
- **CORS**: Configurable CORS support

## Project Structure

```
rust-template/
├── src/
│   ├── config/              # Configuration modules
│   │   ├── mod.rs
│   │   ├── app_config.rs    # Application configuration
│   │   ├── database.rs      # Database configuration
│   │   ├── redis_config.rs  # Redis configuration
│   │   └── mqtt_config.rs   # MQTT configuration
│   │
│   ├── interceptors/        # Response and error interceptors
│   │   ├── mod.rs
│   │   ├── response.rs      # Standard API response format
│   │   └── error.rs         # Error types and handling
│   │
│   ├── middleware/          # Middleware modules
│   │   ├── mod.rs
│   │   ├── auth.rs          # JWT authentication middleware
│   │   └── logging.rs       # Logging setup
│   │
│   ├── models/              # Data models
│   │   ├── mod.rs
│   │   └── user.rs          # User model and DTOs
│   │
│   ├── handlers/            # Request handlers
│   │   ├── mod.rs
│   │   ├── auth_handler.rs  # Authentication endpoints
│   │   ├── user_handler.rs  # User management endpoints
│   │   └── health_handler.rs # Health check endpoint
│   │
│   ├── routes/              # Route definitions
│   │   └── mod.rs
│   │
│   ├── services/            # Business logic services
│   │   ├── mod.rs
│   │   ├── redis_service.rs # Redis operations
│   │   └── mqtt_service.rs  # MQTT operations
│   │
│   ├── queue/               # Queue service
│   │   ├── mod.rs
│   │   ├── job.rs           # Job definitions
│   │   └── queue_service.rs # Queue implementation
│   │
│   ├── utils/               # Utility functions
│   │   ├── mod.rs
│   │   ├── password.rs      # Password hashing/verification
│   │   └── validation.rs    # Request validation
│   │
│   └── main.rs              # Application entry point
│
├── migrations/              # Database migrations
│   └── 20240101000001_create_users_table.sql
│
├── scripts/                 # Utility scripts
│   ├── migrate.sh           # Run migrations
│
├── logs/                    # Application logs
├── docs/                    # Documentation
├── .env.example             # Environment variables example
└── Cargo.toml               # Project dependencies

```

## Quick Start

### Prerequisites

- Rust 1.70+ ([Install Rust](https://www.rust-lang.org/tools/install))
- Docker & Docker Compose ([Install Docker](https://docs.docker.com/get-docker/))

### Installation

1. Clone the repository:
```bash
git clone <repository-url>
cd rust-template
```

2. Copy environment file:
```bash
cp .env.example .env
```

3. Start Docker services (PostgreSQL, Redis, MQTT):
```bash
docker-compose up -d
```

4. Install sqlx-cli (for migrations):
```bash
cargo install sqlx-cli --no-default-features --features postgres
```

5. Run database migrations:
```bash
./scripts/migrate.sh
```

6. Build and run the application:
```bash
cargo run
```

The server will start at `http://localhost:3000` (or the port specified in `.env`).

## Environment Variables

See [.env.example](.env.example) for all available configuration options.

Key variables:
- `DATABASE_URL`: PostgreSQL connection string
- `REDIS_HOST`, `REDIS_PORT`: Redis connection details
- `JWT_SECRET`: Secret key for JWT token generation
- `MQTT_BROKER`: MQTT broker connection string

## API Endpoints

### Public Endpoints (No Authentication)

#### Health Check
```
GET /health
```

**Response:**
```json
{
  "success": true,
  "message": "Service is healthy",
  "data": {
    "status": "ok",
    "timestamp": "2024-01-01T00:00:00Z"
  }
}
```

#### Register
```
POST /auth/register
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "password123",
  "name": "John Doe"
}
```

**Response:**
```json
{
  "success": true,
  "message": "User registered successfully",
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "user": {
      "id": "uuid",
      "email": "user@example.com",
      "name": "John Doe",
      "is_active": true,
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-01T00:00:00Z"
    }
  }
}
```

#### Login
```
POST /auth/login
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "password123"
}
```

**Response:** Same as register

### Protected Endpoints (Require Authentication)

All protected endpoints require the `Authorization` header:
```
Authorization: Bearer <your-jwt-token>
```

#### Get Current User
```
GET /user
```

#### Update User
```
PUT /user
Content-Type: application/json

{
  "email": "newemail@example.com",
  "name": "New Name",
  "is_active": true
}
```

#### Delete User
```
DELETE /user
```

## API Response Format

### Success Response
```json
{
  "success": true,
  "message": "Operation successful",
  "data": { ... }
}
```

### Error Response
```json
{
  "success": false,
  "message": "Error message",
  "error": {
    "code": "ERROR_CODE",
    "details": { ... }
  }
}
```

## Database Migrations

### Run Migrations

To run all pending migrations:
```bash
./scripts/migrate.sh
```

Or manually:
```bash
sqlx migrate run
```

### Migration Best Practices

1. **Database First**: Always run migrations separately before starting the application
2. **Reversible**: Consider creating both up and down migrations
3. **Atomic**: Keep migrations atomic and focused on one change
4. **Test**: Test migrations on a development database first

## Using the Queue Service

The Queue Service is similar to BeeQueue in Node.js and provides reliable job processing with Redis.

### Example: Order Processing Queue

```rust
use crate::queue::QueueService;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrderRequest {
    order_id: String,
    user_id: String,
    amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrderResult {
    order_id: String,
    status: String,
}

pub struct OrderService {
    order_queue: QueueService<OrderRequest, OrderResult>,
}

impl OrderService {
    pub async fn new() -> Result<Self, AppError> {
        let order_queue = QueueService::new("order", 3).await?;

        // Setup queue processor
        let queue_clone = order_queue.clone();
        tokio::spawn(async move {
            queue_clone
                .handle_process_queue(|job| async move {
                    Self::process_create_order(job.data).await
                })
                .await
        });

        Ok(Self { order_queue })
    }

    pub async fn create_order(&self, request: OrderRequest) -> Result<Job<OrderRequest>, AppError> {
        self.order_queue.add_to_queue(request).await
    }

    async fn process_create_order(data: OrderRequest) -> Result<OrderResult, AppError> {
        // Process the order
        tracing::info!("Processing order: {}", data.order_id);

        // Your business logic here
        // ...

        Ok(OrderResult {
            order_id: data.order_id,
            status: "completed".to_string(),
        })
    }
}
```

### Queue Features

- **Automatic Retries**: Failed jobs are automatically retried with exponential backoff
- **Job Persistence**: Jobs are stored in Redis
- **Concurrency Control**: Process one job at a time (configurable)
- **Job Tracking**: Track job status and results
- **Timeout Support**: Jobs have configurable timeouts

## Using Redis Service

```rust
use crate::services::RedisService;

let redis = RedisService::new().await?;

// Simple key-value operations
redis.set("key", "value").await?;
let value = redis.get("key").await?;

// With expiration
redis.set_ex("key", "value", 3600).await?; // 1 hour

// JSON operations
redis.set_json("user:123", &user).await?;
let user: User = redis.get_json("user:123").await?.unwrap();

// Cache operations with prefix
redis.cache_set_json("users", "123", &user, 3600).await?;
let cached: Option<User> = redis.cache_get_json("users", "123").await?;

// List operations
redis.rpush("queue", "item").await?;
let item = redis.lpop("queue").await?;
```

## Using MQTT Service

```rust
use crate::services::MqttService;

let mqtt = MqttService::new().await?;

// Subscribe to topic
mqtt.subscribe("sensors/temperature").await?;

// Publish message
mqtt.publish("sensors/temperature", "25.5", false).await?;

// Publish JSON
mqtt.publish_json("sensors/data", &sensor_data, false).await?;

// Listen for messages
mqtt.listen("sensors/#", |topic, payload| {
    println!("Received on {}: {:?}", topic, payload);
}).await?;
```

## Adding a New API Endpoint

Follow these steps to add a new API endpoint:

### 1. Create/Update Model (if needed)

In `src/models/`, create or update your model:

```rust
// src/models/product.rs
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub price: f64,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateProductRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,

    #[validate(range(min = 0.0))]
    pub price: f64,
}
```

### 2. Create Handler

In `src/handlers/`, create your handler:

```rust
// src/handlers/product_handler.rs
use axum::{extract::State, Json};
use sqlx::PgPool;

use crate::interceptors::{ApiSuccess, AppError};
use crate::models::{CreateProductRequest, Product};
use crate::utils::validate_request;

pub async fn create_product(
    State(pool): State<PgPool>,
    Json(request): Json<CreateProductRequest>,
) -> Result<ApiSuccess<Product>, AppError> {
    validate_request(&request)?;

    let product = sqlx::query_as::<_, Product>(
        "INSERT INTO products (id, name, price) VALUES ($1, $2, $3) RETURNING *"
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&request.name)
    .bind(request.price)
    .fetch_one(&pool)
    .await?;

    Ok(ApiSuccess::new("Product created successfully", product))
}
```

### 3. Add Route

In `src/routes/mod.rs`, add your route:

```rust
use crate::handlers::create_product;

let product_routes = Router::new()
    .route("/products", post(create_product))
    .route_layer(middleware::from_fn(JwtMiddleware::auth)); // If authentication required

// Merge with other routes
Router::new()
    .merge(public_routes)
    .merge(protected_routes)
    .merge(product_routes)
    .with_state(pool)
```

### 4. Create Migration (if needed)

Create the migration file:

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
./scripts/migrate.sh
```

### 5. Test Your Endpoint

```bash
curl -X POST http://localhost:3000/products \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{"name": "Product Name", "price": 99.99}'
```

## Testing

Run tests:
```bash
cargo test
```

Run tests with output:
```bash
cargo test -- --nocapture
```

## Logging

Logs are written to both console and file (configured in `.env`).

Log levels: `trace`, `debug`, `info`, `warn`, `error`

```rust
tracing::info!("Information message");
tracing::error!("Error message: {}", error);
tracing::debug!("Debug data: {:?}", data);
```

## Production Deployment

1. Build release binary:
```bash
cargo build --release
```

2. Set environment to production:
```bash
export ENVIRONMENT=production
```

3. Use a process manager (e.g., systemd, PM2, or supervisord)

4. Configure reverse proxy (e.g., Nginx)

5. Enable HTTPS with proper certificates

6. Set strong `JWT_SECRET` in production

7. Configure proper database connection pooling

8. Setup monitoring and alerting

## License

MIT License - See LICENSE file for details

## Support

For issues and questions, please open an issue on the repository.

## Author
binhtruong9418 - Duc Binh - [GitHub](https://github.com/binhtruong9418)
