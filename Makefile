.PHONY: help build run dev test clean migrate create-migration docker-up docker-down install lint format check

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

install: ## Install dependencies and tools
	@echo "Installing sqlx-cli..."
	cargo install sqlx-cli --no-default-features --features postgres
	@echo "Done!"

build: ## Build the project in release mode
	@echo "Building project..."
	cargo build --release

dev: ## Run the project in development mode with auto-reload
	@echo "Running in development mode..."
	cargo watch -x run

run: ## Run the project
	@echo "Running project..."
	cargo run

test: ## Run all tests
	@echo "Running tests..."
	cargo test

test-verbose: ## Run tests with output
	@echo "Running tests with output..."
	cargo test -- --nocapture

clean: ## Clean build artifacts
	@echo "Cleaning build artifacts..."
	cargo clean
	rm -rf logs/*.log

migrate: ## Run database migrations
	@echo "Running migrations..."
	./scripts/migrate.sh

lint: ## Run clippy linter
	@echo "Running clippy..."
	cargo clippy -- -D warnings

format: ## Format code with rustfmt
	@echo "Formatting code..."
	cargo fmt

format-check: ## Check code formatting
	@echo "Checking code format..."
	cargo fmt -- --check

check: lint format-check ## Run all checks (lint + format)
	@echo "All checks passed!"

docker-up: ## Start Docker services (PostgreSQL, Redis, MQTT)
	@echo "Starting Docker services..."
	docker-compose up -d

docker-down: ## Stop Docker services
	@echo "Stopping Docker services..."
	docker-compose down

docker-logs: ## Show Docker logs
	docker-compose logs -f

setup: install ## Setup development environment
	@echo "Setting up development environment..."
	cp .env.example .env
	@echo "Please edit .env file with your configuration"
	@echo "Then run: make migrate"

prod-build: ## Build for production
	@echo "Building for production..."
	cargo build --release
	@echo "Binary is at: ./target/release/rust-backend-template"

watch: ## Watch for changes and rebuild
	@echo "Watching for changes..."
	cargo watch -x check -x test -x run
