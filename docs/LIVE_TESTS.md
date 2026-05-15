# Live Test Credential Setup

This guide walks you through getting credentials for all three platforms so you can run the live tests.

---

## Step 1 — Start PeerTube (Docker)

**Do this first.** It's the easiest platform to set up and validates the whole test infrastructure.

```bash
cd /home/dracon/Dev/video-uploader/docker/peertube
docker compose up -d
```

Wait ~15 seconds for health checks, then verify:
```bash
curl -s http://localhost:9000 | head -5
```

Check logs if it doesn't come up:
```bash
docker compose logs -f peertube
```

**Teardown** (when done testing):
```bash
docker compose down -v
```

---

## Step 2 — Create PeerTube Account + Get API Token

### 2a. Register an account

Open **http://localhost:9000** in your browser.

1. Click **"Register"** (or "Sign up")
2. Fill in:
   - Username: `testadmin`
   - Email: `test@example.com`
   - Password: `testpass123` (or whatever you like)
3. Submit → You should be logged in

> If you see "Email is already reserved" someone already registered it. Pick a different email.

### 2b. Make yourself admin (so you can create an API token)

```bash
docker compose exec peertube node dist/scripts/create-admin.js \
  -u test@example.com -p testpass123
```

You should see: `Admin user test@example.com created.`

### 2c. Create an API token

1. Log in at **http://localhost:9000**
2. Click your avatar (top right) → **Settings**
3. In the left sidebar: **Authorization** → **Applications**
4. Click **"Create a new application"**
5. Fill in:
   - Name: `video-uploader-cli`
   - Redirect URI: leave default (not used)
   - Scopes: check `api` (or just keep defaults — the plugin manages this)
6. Click **"Create"**
7. **Copy the token** — it looks like `eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...`

> The token is shown only once. If you lose it, delete the application and create a new one.

---

## Step 3 — Configure `.env.test`

Copy the example and fill it in:
```bash
cp /home/dracon/Dev/video-uploader/.env.test.example /home/dracon/Dev/video-uploader/.env.test
```

Edit `.env.test`:
```env
# PeerTube (P0 - easiest)
PEERTUBE_TEST_INSTANCE_URL=http://localhost:9000
PEERTUBE_TEST_TOKEN=your_token_here         # ← paste the token from Step 2c
PEERTUBE_ALLOW_HTTP=1
```

---

## Step 4 — Run PeerTube Live Tests

```bash
cd /home/dracon/Dev/video-uploader
cargo test -p video-uploader --features live-test --test live_peertube -- --test-threads=1
```

Expected output:
```
running 2 tests
test test_peertube_upload_and_delete ... ok
test test_peertube_upload_private_visibility ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

---

## (Optional) YouTube Auth — Get a Refresh Token

The refresh token is a one-time setup. After you get it, it's stored in the encrypted credential file and auto-refreshes forever.

### Step A. Create OAuth credentials in Google Cloud Console

1. Go to **https://console.cloud.google.com/apis/credentials**
2. Sign in with your Google account
3. Click **"Create Credentials"** → **OAuth client ID**
4. Application type: **Desktop app** (or "Other" if Desktop isn't an option)
5. Name: `video-uploader`
6. Click **"Create"**
7. Copy your **Client ID** and **Client Secret**

### Step B. Authenticate using the CLI

```bash
cd /home/dracon/Dev/video-uploader
cargo run -p video-uploader-cli -- \
  --passphrase your-passphrase-here \
  auth --platform youtube \
  --client-id YOUR_CLIENT_ID \
  --client-secret YOUR_CLIENT_SECRET
```

The CLI will print:
```
===========================================
  IMPORTANT: One-time YouTube authorization
===========================================

  1. Open this URL on any device:
     https://www.google.com/device

  2. Enter this code: XXXX-XXXX

  Waiting for authorization...
```

### Step C. Authorize in your browser

1. Open the URL shown (`https://www.google.com/device`)
2. Enter the code shown
3. Sign in with the Google account you want to use for uploads
4. Click **"Allow"** for the permissions (access YouTube, upload videos)
5. Wait — the CLI will poll until you complete the flow

Once authorized, you'll see:
```
YouTube credentials saved successfully!
```

The refresh token is now stored in `~/.config/video-uploader/credentials.enc`.

### Step D. Add to `.env.test`

```env
# YouTube (P1/P2)
YOUTUBE_TEST_CLIENT_ID=your_client_id
YOUTUBE_TEST_CLIENT_SECRET=your_client_secret
YOUTUBE_TEST_REFRESH_TOKEN=  # leave blank — stored in encrypted creds file
```

To get the stored refresh token value (for wiremock test env vars):
```bash
cat ~/.config/video-uploader/credentials.enc
# It's encrypted, so you can't read it directly.
# Instead, just set the URL overrides directly in wiremock tests (done already).
# You only need this if running live YouTube tests.
```

### Run YouTube live tests

```bash
cargo test -p video-uploader --features live-test --test live_youtube -- --test-threads=1
```

---

## (Optional) Odysee — Start lbrynet Daemon

### Step A. Install lbry-sdk

```bash
# Requires Python 3.8+ and pip
pip install lbry-sdk
```

### Step B. Start the daemon

```bash
lbrynet start
```

Wait for it to be ready (~10s). Verify:
```bash
curl -s http://localhost:5279/lbrynet -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"status"}' | jq .
```

Should return something like:
```json
{"jsonrpc":"2.0","id":1,"result":{"lbrynet_version":"1.88.0",...}}
```

### Step C. (Optional) Create a channel

```bash
lbrynet channel new --name @mychannel
# Follow prompts to reserve the name
```

### Step D. Add to `.env.test`

```env
# Odysee (P3)
ODYSEE_TEST_DAEMON_URL=http://localhost:5279
ODYSEE_TEST_CHANNEL_NAME=@mychannel   # optional, omit if no channel
ODYSEE_ALLOW_REMOTE_DAEMON=0
```

### Run Odysee live tests

```bash
cargo test -p video-uploader --features live-test --test live_odysee -- --test-threads=1
```

---

## Wiremock Integration Tests

The wiremock tests use the `test-utils` feature to access URL-override helpers:

```bash
# Run all wiremock tests (17 tests, default threading — no env race)
cargo test -p video-uploader --test wiremock --features test-utils

# Run only the auth URL override tests
cargo test -p video-uploader --test wiremock --features test-utils \
  -- test_refresh_access_token test_start_device_code
```

Without `--features test-utils`, the wiremock tests fail to compile (the URL-override helpers are gated behind that feature).

---

## Quick Reference — What You Need Per Platform

| Platform | What to get | Where | Live test command |
|---|---|---|---|
| **PeerTube** | API token (string) | http://localhost:9000 → Settings → Authorization → Applications | `cargo test --features live-test --test live_peertube` |
| **YouTube** | Client ID + Client Secret (OAuth) | https://console.cloud.google.com/apis/credentials | `cargo test --features live-test --test live_youtube` |
| **Odysee** | lbrynet running at localhost:5279 | https://github.com/lbryio/lbry-sdk | `cargo test --features live-test --test live_odysee` |
| **Wiremock** | (no credentials needed) | — | `cargo test --features test-utils --test wiremock` |

---

## Troubleshooting

### PeerTube: "Connection refused" on port 9000
```bash
docker compose ps    # is the container running?
docker compose logs peertube  # any errors on startup?
# Restart:
docker compose down && docker compose up -d
```

### PeerTube: Token not working
Make sure you're using the **raw token string** (not URL-encoded). If you created the app but can't see the token, the app might not have been created. Go to Settings → Administration → OAuth2 Applications to verify.

### YouTube: "invalid_client" error
Check that your OAuth client is set to **Desktop app** type (not Web application). Desktop clients use device code flow which is different from web OAuth.

### YouTube: Device code flow times out
The code expires after ~10 minutes. If you took too long, just run the CLI again — it generates a fresh code.

### Odysee: Daemon not responding
```bash
lbrynet status   # check daemon status
lbrynet stop && lbrynet start  # restart
```

### All tests: "env var not set" panic
Check that `.env.test` exists at `/home/dracon/Dev/video-uploader/.env.test` and that the required keys are set (no empty values after the `=`).