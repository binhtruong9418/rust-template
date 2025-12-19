#!/bin/bash

# Database Migration Script
# This script runs database migrations using sqlx-cli

set -e

echo "Running database migrations..."

# Load environment variables
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

# Check if DATABASE_URL is set
if [ -z "$DATABASE_URL" ]; then
    echo "Error: DATABASE_URL is not set"
    echo "Please set DATABASE_URL in your .env file"
    exit 1
fi

# Install sqlx-cli if not installed
if ! command -v sqlx &> /dev/null; then
    echo "sqlx-cli not found. Installing..."
    cargo install sqlx-cli --no-default-features --features postgres
fi

# Run migrations
echo "Running migrations on: $DATABASE_URL"
sqlx migrate run

echo "Migrations completed successfully!"
