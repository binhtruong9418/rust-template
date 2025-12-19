# Quick Start Guide

Get the Rust Backend Template up and running in 5 minutes!

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) 1.70+
- PostgreSQL 12+ (for database)
- Redis 6+ (for caching and queue)
- (Optional) MQTT Broker like Mosquitto (for IoT features)

## Quick Setup

### 1. Clone and Configure

```bash
# Clone the repository
git clone <your-repo-url>
cd rust-template

# Copy environment file
cp .env.example .env
```

### 2. Configure Environment

Edit `.env` with your settings:

```env
# Server
HOST=0.0.0.0
PORT=3000
ENVIRONMENT=development

# Database - Update with your PostgreSQL credentials
DATABASE_URL=postgresql://username:password@localhost:5432/rust_template_db
DATABASE_MAX_CONNECTIONS=10

# Redis - Update if using password
REDIS_HOST=localhost
REDIS_PORT=6379
REDIS_PASSWORD=
REDIS_DB=0
REDIS_POOL_SIZE=10

# JWT
JWT_SECRET=your-secret-key-change-this
JWT_EXPIRATION=86400

# MQTT (optional)
MQTT_BROKER=mqtt://localhost:1883
MQTT_CLIENT_ID=rust-backend-template
MQTT_USERNAME=
MQTT_PASSWORD=

# Logging
LOG_LEVEL=debug
LOG_FILE=logs/app.log
```

### 3. Setup Database

Create the PostgreSQL database:

```bash
# Using psql
createdb rust_template_db

# Or with PostgreSQL user
sudo -u postgres createdb rust_template_db
```

### 4. Install SQLx CLI and Run Migrations

```bash
# Install SQLx CLI
cargo install sqlx-cli --no-default-features --features postgres

# Run migrations
sqlx migrate run
```

### 5. Run the Application

```bash
cargo run
```

Your server is now running at `http://localhost:3000`! 🚀

## Testing the API

### 1. Health Check

```bash
curl http://localhost:3000/health
```

Expected response:
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

### 2. Register a User

```bash
curl -X POST http://localhost:3000/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "password123",
    "name": "Test User"
  }'
```

You'll receive a JWT token in the response. Save it!

### 3. Get User Profile

```bash
# Replace YOUR_TOKEN with the token from registration
curl http://localhost:3000/user \
  -H "Authorization: Bearer YOUR_TOKEN"
```

## Alternative: Using Docker (Optional)

If you prefer Docker for development dependencies:

### 1. Start Services with Docker

```bash
# Start PostgreSQL, Redis, and MQTT using Docker
docker-compose up -d

# Wait for services to be ready
sleep 5
```

### 2. Use Default Docker Configuration

The `.env.example` already has Docker-friendly defaults:

```env
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/rust_template_db
REDIS_HOST=localhost
REDIS_PORT=6379
MQTT_BROKER=mqtt://localhost:1883
```

### 3. Run Migrations and Application

```bash
# Run migrations
sqlx migrate run

# Run application
cargo run
```

## Using Makefile (Even Easier!)

If you have `make` installed:

```bash
# View all available commands
make help

# Start Docker services (if using Docker)
make docker-up

# Run migrations
make migrate

# Run in development mode with auto-reload
make dev

# Or just run normally
make run
```

## What's Next?

### Learn the API

Check the API endpoints section in [README.md](README.md#api-endpoints) for complete documentation.

### Add Your First Endpoint

Follow the guide in [README.md](README.md#adding-a-new-api-endpoint)

### Deploy to Production

When you're ready: [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md)

## Common Issues

### Port Already in Use

If port 3000 is already taken, change it in `.env`:
```env
PORT=8080
```

### Database Connection Error

Make sure PostgreSQL is running:

```bash
# Check PostgreSQL status
systemctl status postgresql     # Linux
brew services list               # macOS
pg_isready                       # Test connection
```

Create the database if it doesn't exist:
```bash
createdb rust_template_db
```

### Redis Connection Error

Check if Redis is running:

```bash
# Check Redis status
redis-cli ping    # Should return PONG

# Start Redis if needed
systemctl start redis    # Linux
brew services start redis  # macOS
```

### Migration Errors

Reset and try again:

```bash
# Drop and recreate database
dropdb rust_template_db
createdb rust_template_db

# Run migrations again
sqlx migrate run
```

### SQLx Offline Mode (for CI/CD)

If you get SQLx compile errors:

```bash
# Prepare offline mode
cargo sqlx prepare

# This creates sqlx-data.json for offline compilation
```

## Development Tips

### Auto-reload on Code Changes

```bash
# Install cargo-watch
cargo install cargo-watch

# Run with auto-reload
cargo watch -x run

# Or use make
make dev
```

### View Logs

Logs are written to both console and `logs/app.log`.

Adjust log level in `.env`:
```env
LOG_LEVEL=debug  # trace, debug, info, warn, error
```

### API Testing Tools

- [Postman](https://www.postman.com/)
- [Insomnia](https://insomnia.rest/)
- [HTTPie](https://httpie.io/)
- Thunder Client (VS Code extension)

## Installing Services

### PostgreSQL

**macOS:**
```bash
brew install postgresql@15
brew services start postgresql@15
```

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install postgresql postgresql-contrib
sudo systemctl start postgresql
```

**Windows:**
Download from [postgresql.org](https://www.postgresql.org/download/windows/)

### Redis

**macOS:**
```bash
brew install redis
brew services start redis
```

**Ubuntu/Debian:**
```bash
sudo apt install redis-server
sudo systemctl start redis-server
```

**Windows:**
Download from [redis.io/download](https://redis.io/download) or use WSL

### MQTT (Optional)

**macOS:**
```bash
brew install mosquitto
brew services start mosquitto
```

**Ubuntu/Debian:**
```bash
sudo apt install mosquitto mosquitto-clients
sudo systemctl start mosquitto
```

## Project Structure

```
rust-template/
├── src/
│   ├── config/          # Configuration
│   ├── handlers/        # API endpoints
│   ├── interceptors/    # Response/Error handling
│   ├── middleware/      # Auth, logging
│   ├── models/          # Data models
│   ├── queue/           # Queue service
│   ├── routes/          # URL routing
│   ├── services/        # Redis, MQTT
│   ├── utils/           # Helper functions
│   └── main.rs          # Entry point
├── migrations/          # Database migrations
├── docs/                # Documentation
└── scripts/             # Utility scripts
```

## Support

- **Issues**: Check [GitHub Issues](https://github.com/your-repo/issues)
- **Documentation**: See [README.md](README.md) and [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md)

Happy coding! 🦀
