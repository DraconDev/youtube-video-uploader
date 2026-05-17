# video-uploader — Project Specification

**Version**: 0.2.0 (target)  
**Date**: 2026-05-17

---

## Goal

Upload videos to **YouTube** via the **YouTube Data API v3** (resumable upload). Support **multiple channels** (workspaces) so a single installation can manage and upload to several YouTube accounts from one machine.

## Scope

### In Scope
- **YouTube only** — Other platforms (Odysee, Rumble, PeerTube) were evaluated and excluded: too small an audience or no usable upload API.
- **API-only path** — YouTube Data API v3 resumable upload. The browser-automation path (Playwright) is out of scope for now (API quota ≈6 uploads/day is acceptable for current needs).
- **Multiple channels** — Each YouTube channel has its own OAuth2 credentials. The tool must store, select, and operate on named channel profiles.

### Out of Scope
- Browser automation / Playwright upload path
- Non-YouTube platforms
- Video transcoding or processing
- Thumbnail upload (future consideration)
- Scheduled/delayed publishing (future consideration)
- Playlist management

---

## Core Concepts

### Workspace (Channel Profile)

A **workspace** is a named set of YouTube OAuth2 credentials for one channel. A user with three YouTube channels would have three workspaces.

```
~/.config/video-uploader/
├── credentials.enc          # encrypted store (all workspaces in one file)
└── resume/                  # upload resume state (future)
```

Each workspace stores:
| Field | Purpose |
|-------|---------|
| `client_id` | Google OAuth2 client ID |
| `client_secret` | Google OAuth2 client secret |
| `refresh_token` | Long-lived token (per-channel) |
| `access_token` | Short-lived token (auto-refreshed) |
| `token_expires_at` | Expiry timestamp for access token |

### Default Workspace

One workspace is marked as **default** and used when no `--workspace` flag is given. The first workspace created becomes the default automatically.

### Credential Encryption

All credentials are stored in a single AES-256-GCM encrypted file (`credentials.enc`), protected by a user passphrase. Format V2 uses PBKDF2 key derivation (100K iterations, random salt + nonce).

---

## CLI Interface

```
video-uploader [global options] <command>

Global options:
  --passphrase <PASS>          Passphrase for credential store
  --passphrase-file <FILE>     Read passphrase from file
  -w, --workspace <NAME>       Workspace (channel) to use

Commands:
  auth                        Authenticate a YouTube channel
  upload                      Upload a single video
  batch                       Upload videos from a CSV manifest
  list                        List configured workspaces
  workspace                   Manage workspaces (rename, default, remove)
```

### `auth` — Authenticate a channel

```bash
# Interactive: runs OAuth2 device code flow
video-uploader auth --client-id ID --client-secret SECRET
# → Creates workspace named "youtube" (or first free slot)

# Named workspace for a second channel
video-uploader auth --client-id ID --client-secret SECRET -w cooking-channel
```

### `upload` — Upload a video

```bash
# Uses default workspace
video-uploader upload --file video.mp4 --title "My Video"

# Specify workspace
video-uploader upload --file video.mp4 --title "My Video" -w gaming-channel
```

### `batch` — Upload from CSV

```bash
# CSV columns: file, title, description, tags, visibility, workspace (optional)
video-uploader batch --manifest videos.csv --concurrency 4

# workspace column in CSV allows per-row channel selection
# If workspace column is empty/missing, uses --workspace or default
```

### `list` — Show workspaces

```bash
video-uploader list
# Output:
#   Workspaces:
#     - youtube (default)
#     - cooking-channel
#     - gaming-channel
```

### `workspace` — Manage workspaces

```bash
video-uploader workspace default cooking-channel   # set default
video-uploader workspace rename youtube main       # rename a workspace
video-uploader workspace remove old-channel        # delete credentials
```

---

## Library API

### Workspace-Aware CredentialStore

Current `CredentialStore` uses `HashMap<String, PlatformCredentials>` keyed by platform name (`"youtube"`). This becomes the workspace store — keyed by workspace name instead.

```rust
// Before (v0.1): platform-keyed
store.set("youtube", creds);
store.get("youtube");

// After (v0.2): workspace-keyed  
store.set("gaming-channel", creds);
store.get("gaming-channel");
store.default_workspace() -> Option<&str>;
store.set_default("gaming-channel");
```

### YouTubeUploader accepts workspace

```rust
// Before
YouTubeUploader::new(store, &passphrase);

// After — workspace is resolved at construction
YouTubeUploader::new(store, &passphrase, "gaming-channel");
```

`get_access_token()` inside the uploader looks up credentials by workspace name instead of the hardcoded `"youtube"` key.

---

## Data Format (credentials.enc V2)

```toml
default_workspace = "youtube"

[workspaces.youtube]
client_id = "..."
client_secret = "..."
refresh_token = "..."
access_token = "..."
token_expires_at = 1747500000

[workspaces.cooking-channel]
client_id = "..."
client_secret = "..."
refresh_token = "..."
access_token = "..."
token_expires_at = 1747500000
```

### Migration from V0.1

On `load()`, if the credential store contains the old format (top-level platform keys like `[youtube]` without a `[workspaces]` section), auto-migrate:
1. Move `[youtube]` → `[workspaces.youtube]`
2. Set `default_workspace = "youtube"` (top-level key)
3. Re-encrypt and save

This preserves the existing single-channel setup with zero user action.

---

## Upload Flow (unchanged)

```
1. Resolve workspace → get credentials
2. Refresh access token if expired (60s buffer)
3. Validate video file (size, extension, title)
4. Initiate resumable upload → get upload URL
5. Chunk upload (8 MiB chunks, 308 resume support)
6. Return UploadResult { platform, platform_id, url, title }
```

Batch mode adds:
- CSV parsing with optional per-row workspace column
- Semaphore-limited concurrency (--concurrency)
- Per-entry `YouTubeUploader` constructed with the row's workspace (or default)

---

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| YouTube only | Other platforms lack scale or usable upload APIs |
| Single encrypted file | One passphrase, one file — simpler than per-workspace files |
| Workspace names, not numbers | Human-readable, shows in CLI output and CSV |
| Default workspace | Most users have one channel — zero-friction default path |
| API path only | Browser automation is a future phase; API quota is acceptable now |
| `--workspace` / `-w` flag | Short flag for common use; consistent across all commands |

---

## What Needs to Change (from v0.1 → v0.2)

### Library (`video-uploader`)
1. **`config.rs`** — `CredentialStore` keys change from platform names to workspace names. Add `default_workspace` metadata. Add migration from v0.1 format.
2. **`youtube.rs`** — `YouTubeUploader::new()` takes a workspace name. `get_access_token()` looks up by workspace, not hardcoded `"youtube"`.
3. **`upload.rs`** — No changes needed (types are platform-agnostic).
4. **`progress.rs`** — No changes needed.
5. **`error.rs`** — No changes needed.
6. **`validation.rs`** — No changes needed.
7. **`auth/`** — No changes needed (device code flow is workspace-agnostic).

### CLI (`video-uploader-cli`)
1. Add `--workspace` / `-w` global flag to `Cli` struct
2. `auth` command — use workspace name (default `"youtube"`) instead of hardcoding `"youtube"`
3. `upload` command — pass workspace to `YouTubeUploader::new()`
4. `batch` command — read optional `workspace` column from CSV, create per-row uploaders
5. `list` command — show workspace names + default marker
6. Add `workspace` subcommand (default, rename, remove)

### Tests
- Update all `store.set("youtube", ...)` → `store.set("test-workspace", ...)`
- Add migration test (old format → new format)
- Add multi-workspace test (two sets of credentials, verify correct one used)
- Add CLI `--workspace` flag tests
