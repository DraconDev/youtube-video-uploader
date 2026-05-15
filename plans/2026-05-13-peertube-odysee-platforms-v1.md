# Plan: Add PeerTube + Odysee Platform Support

## Context

Goal: maximize compound audience by uploading to every platform with free discovery and a public API.

**Existing:** YouTube (done), api.video (done)
**New targets:**
1. **PeerTube** — federated video platform, growing instances, pure REST API, no daemon
2. **Odysee/LBRY** — blockchain-backed, curation-driven homepage, engaged audience

## Implementation Plan

### Phase 1: PeerTube (Est. 2-3 hours)

#### Task 1.1: Research PeerTube instances and API
- Find public instances that accept sign-up via API (Framatube, etc.)
- Document authentication flow (OAuth2 or API token)
- Identify upload endpoint and response format
- Note: PeerTube is federated — each instance is independent. A video uploaded to one instance federates to all instances that follow it.

#### Task 1.2: Create `src/platforms/peertube.rs`
- `PeerTubeUploader` struct with API token + instance URL
- `create_video()` → POST to instance, returns video ID
- `upload_file()` → PUT multipart to upload URL
- `complete_upload()` → mark video as published
- Implement `PlatformUploader` trait
- Add `mime_guess` for content type
- 60s request timeout on client
- Retry with exponential backoff on 5xx

#### Task 1.3: Wire up in `src/platforms/mod.rs`

#### Task 1.4: Add to `src/registry.rs`
- Add `"peertube"` arm to `get_uploader()`
- Validate credentials (instance URL + token)

#### Task 1.5: Add CLI support
- `video-uploader auth --platform peertube --instance-url https://framatube.org --token <token>`
- `video-uploader upload --platforms peertube --file video.mp4 --title "..."`

#### Task 1.6: Write wiremock tests
- Upload success (201 Created)
- Auth failure (401)
- Instance unreachable (5xx → retry)

#### Task 1.7: Add to README

---

### Phase 2: Odysee/LBRY (Est. 4-6 hours)

#### Task 2.1: Research LBRY SDK API
- Determine if LBRY SDK can run headless (no display)
- Document `lbrynet` binary installation/fetching
- Document `wallet_balance` and `channel_list` API calls needed
- Research if there's a hosted/remote daemon option
- Alternative: check if Odysee.com has a proxy API

#### Task 2.2: Architecture Decision
Two paths:
- **Path A:** Spawn `lbrynet` as a subprocess, manage lifecycle from Rust
- **Path B:** Require `lbrynet` to be pre-installed and running
- **Path C:** Use LBRY SDK's HTTP API over a configurable daemon URL
- Choose based on research from Task 2.1

#### Task 2.3: Create `src/platforms/lbry.rs`
- `LbryUploader` struct with daemon URL + channel name
- `publish()` → JSON-RPC call to daemon at `http://localhost:5279`
- Map VideoUpload fields to LBRY publish params
- Handle blockchain confirmation (poll for claim result)
- Implement `PlatformUploader` trait
- Implement retry with backoff

#### Task 2.4: Add to `src/platforms/mod.rs`

#### Task 2.5: Add to `src/registry.rs`
- Add `"odysee"` and `"lbry"` arms to `get_uploader()`

#### Task 2.6: Add CLI support
- `video-uploader auth --platform odysee --daemon-url http://localhost:5279`
- Handle daemon-not-running gracefully with install instructions
- `video-uploader upload --platforms odysee --file video.mp4 --title "..."`

#### Task 2.7: Write integration tests
- Mock JSON-RPC responses with wiremock

#### Task 2.8: Add to README

---

## Verification Criteria

- [ ] `cargo test` passes — 71+ tests
- [ ] `cargo clippy` — zero warnings
- [ ] `PeerTubeUploader::new()` and `upload()` work end-to-end
- [ ] CLI `auth` stores PeerTube credentials in encrypted store
- [ ] CLI `upload --platforms peertube` uploads to configured instance
- [ ] Odysee: daemon detection and clear error message if not running
- [ ] Odysee: `publish()` calls daemon JSON-RPC successfully
- [ ] Both platforms show up in `video-uploader list`
- [ ] README updated with setup instructions for both platforms

## Alternative Approaches

### For Odysee
1. **Subprocess** (`lbrynet` as child process) — handles lifecycle but complex
2. **External daemon** (user installs `lbrynet`) — simplest Rust integration
3. **Odysee.com API proxy** — undocumented but potentially simpler
4. **Skip Odysee** — if daemon complexity is too high, prioritize others

### For PeerTube
- One instance at a time (no multi-instance upload without custom logic)
- Could extend to upload to multiple instances by calling same uploader multiple times

## Risks

1. **PeerTube instances go down** — handle gracefully with instance URL config
2. **LBRY daemon complexity** — may need to be optional/deferred
3. **No monetization** — both are free platforms, returns are engagement not revenue

## Out of Scope

- Monetization/transactions on LBRY
- Running a PeerTube instance
- Managing LBRY wallet/funds
