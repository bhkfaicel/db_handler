#!/bin/bash

# Script to run tests in Docker
set -e

# Load .env from project root (directory containing docker-compose.test.yml)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
if [ -f "$PROJECT_ROOT/.env" ]; then
  set -a
  source "$PROJECT_ROOT/.env"
  set +a
fi

echo "🐳 Starting Docker environment for tests..."

# Stop existing containers if they exist
docker-compose -f docker-compose.test.yml down -v

# CARGO_REGISTRIES_FORGEJO_TOKEN required when Cargo.toml uses db_derive from registry "forgejo"
if [ -z "${CARGO_REGISTRIES_FORGEJO_TOKEN}" ]; then
    echo "⚠️  CARGO_REGISTRIES_FORGEJO_TOKEN is not set. Docker build needs it to fetch db_derive from Forgejo."
    echo "   Set it in .env or: export CARGO_REGISTRIES_FORGEJO_TOKEN=your_read_only_token"
    exit 1
fi

# Build and start services
echo "📦 Building Docker images..."
docker-compose -f docker-compose.test.yml build

echo "🚀 Starting services (InfluxDB)..."
docker-compose -f docker-compose.test.yml up -d influxdb

echo "⏳ Waiting for InfluxDB to be ready (Docker healthcheck)..."
# Docker Compose healthcheck handles the wait automatically via depends_on
# We just wait a bit to be sure
sleep 3

echo "✅ InfluxDB is ready!"
echo ""
echo "🧪 Running tests..."
docker-compose -f docker-compose.test.yml run --rm test-runner

# Get exit code
exit_code=$?

# Cleanup (optional - comment out if you want to keep containers for debugging)
if [ "$1" != "--keep" ]; then
    echo ""
    echo "🧹 Cleaning up..."
    docker-compose -f docker-compose.test.yml down -v
fi

exit $exit_code
