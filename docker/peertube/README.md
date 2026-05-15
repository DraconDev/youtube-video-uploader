# PeerTube Test Instance

Run a local PeerTube instance for live testing.

## Setup

```bash
cd docker/peertube
docker compose up -d
```

Wait for PeerTube to be healthy (check logs with `docker compose logs -f peertube`).

## Create Admin Account

1. Open http://localhost:9000
2. Register a new account
3. Make it an admin: `docker compose exec peertube node dist/scripts/create-admin.js -u test@example.com -p testpass123`

## Get API Token

1. Log in at http://localhost:9000
2. Settings → Authorization → Applications
3. Create a new application with a name like `test-cli`
4. Copy the token

## Configure Test Environment

```bash
cp ../../.env.test.example ../../.env.test
# Edit .env.test and set:
PEERTUBE_TEST_TOKEN=your_token_here
PEERTUBE_TEST_INSTANCE_URL=http://localhost:9000
PEERTUBE_ALLOW_HTTP=1
```

## Run Live Tests

```bash
cargo test --features live-test --test live_peertube
```

## Teardown

```bash
docker compose down -v  # -v removes volumes (all data)
```