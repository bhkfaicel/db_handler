# 🐳 Running tests in Docker

This guide explains how to run all tests in an isolated Docker environment.

## 📋 Prerequisites

- Docker and Docker Compose installed
- No other system dependencies required

## 🚀 Quick usage

### Run all tests

```bash
./scripts/test-docker.sh
```

### Run tests and keep containers for debugging

```bash
./scripts/test-docker.sh --keep
```

## 🔧 Docker services

The `docker-compose.test.yml` file defines two services:

1. **influxdb**: InfluxDB 2.7 instance used for history tests

   - Port: `8086`
   - Organization: `test-org`
   - Bucket: `test_bucket`
   - Token: `test-token-...`

2. **test-runner**: Rust container that runs the tests
   - Uses the image built from `Dockerfile.test`
   - Connects to InfluxDB through the Docker network

## 📝 Configuration

The `config.test.json` file is used as the configuration for tests in Docker:

- SQLite database: `/tmp/home_iot_test.db` (in memory for tests)
- InfluxDB: `http://influxdb:8086` (the Docker service name)

## 🛠️ Manual commands

### Start only InfluxDB

```bash
docker-compose -f docker-compose.test.yml up -d influxdb
```

### Run tests manually

```bash
docker-compose -f docker-compose.test.yml run --rm test-runner
```

### View InfluxDB logs

```bash
docker-compose -f docker-compose.test.yml logs influxdb
```

### Stop and clean everything

```bash
docker-compose -f docker-compose.test.yml down -v
```

## 🔍 Debugging

If tests fail:

1. **Check InfluxDB logs**:

   ```bash
   docker-compose -f docker-compose.test.yml logs influxdb
   ```

2. **Check test-runner logs**:

   ```bash
   docker-compose -f docker-compose.test.yml logs test-runner
   ```

3. **Get a shell inside the test-runner container**:

   ```bash
   docker-compose -f docker-compose.test.yml run --rm test-runner bash
   ```

4. **Keep containers after tests**:

   ```bash
   ./scripts/test-docker.sh --keep
   ```

## 📦 Docker caches

Docker volumes are used for:

- `cargo-cache`: Rust dependency cache
- `target-cache`: Rust build cache
- `influxdb_test_data`: InfluxDB data (wiped on every `down -v`)

## ⚙️ Environment variables

The following environment variables are defined in `docker-compose.test.yml`:

- `CONFIG_FILE=/app/config.test.json`
- `TIMESERIES_DB_URL=http://influxdb:8086`
- `TIMESERIES_DB_ORG=test-org`
- `TIMESERIES_DB_TOKEN=test-token-...`
- `SQLX_OFFLINE=true`

## 🎯 Benefits

- ✅ Isolated and reproducible environment
- ✅ No need to install Rust, InfluxDB, etc. locally
- ✅ Same environment for all developers
- ✅ Easy to plug into CI/CD
- ✅ Automatic cleanup after tests
