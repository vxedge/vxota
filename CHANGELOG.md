# Changelog

All notable changes to this repo are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning follows
[Semantic Versioning](https://semver.org/) once the first tag is cut (see
`docs/VERSIONING.md` in the `docs` repo — no `v0.1.0` yet, pre-phase-1-gate).

## [Unreleased]

### Added
- Initial 3-crate workspace: `vxota-common`, `vxota` (server), `vxota-probe` (client) —
  vxota v1 core (2026-07-12).
- `vxota-common::store`: content-addressed artifact store (SHA256), atomic writes.
- `vxota-common::metadata`: Uptane-flavored signed root/targets metadata (vocabulary/format
  only, no aktualizr dependency — `ARCHITECTURE.md` §8); ed25519 signing/verification with
  full trust-chain semantics (root pinned out-of-band, targets verified against the key
  root vouches for).
- `vxota keygen`/`publish`/`serve`: root-key generation, sign-and-publish, and HTTP delivery
  with resumable (`Range`) downloads via `tower_http::ServeDir`.
- `vxota-probe fetch`: trust-chain verification + resumable download + digest check;
  `--repeat`/`--interval-s` drives a sustained H4 experiment window; live Prometheus
  deployment-success/delivery-latency metrics, mirroring `vxprobe`'s RTT exporter.
- CI: build/test/clippy/fmt, `cargo-deny` (advisories/bans/licenses/sources all pass).
- Loopback end-to-end test: keygen → publish → serve → fetch/verify, plus wrong-trust-root
  and unknown-target rejection tests.

### Fixed
- `vxota-probe`'s resumable download appended to a partial file without confirming the
  server actually answered `206 Partial Content` — a server that ignored the `Range`
  header would have had its full-content response corrupt-appended onto existing bytes;
  now restarts clean instead when the response isn't actually partial (found before
  release, during initial development).
- A missing `flush()` before the immediately-following digest check could read back a
  file tokio hadn't finished making visible on disk — found by the loopback e2e test
  failing with a "digest mismatch against SHA256 of empty bytes" on first run.

### Known gaps
- Container image (digest-tagged, non-root) + `kind: container` service descriptor
  example; CI publish to GHCR.
- `vxedge`-side wiring so `vxedge ota up --experiment <manifest>` spawns/manages this
  server from the manifest's `services.ota` block.
