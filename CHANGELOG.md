# Changelog

All notable changes to this repo are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning follows
[Semantic Versioning](https://semver.org/) — see `docs/VERSIONING.md` in the `docs` repo.

## [Unreleased]

### Security

- Lockfile update `tokio` 1.52.3 -> 1.54.1, which fixes RUSTSEC-2026-0386 (unsound `AsyncFd` constructors). vxota does not call `AsyncFd` directly.

## [0.1.1] - 2026-10-22

### Added

- Tag-triggered release workflow (`.github/workflows/release.yml`): builds `vxota` and `vxota-probe` for `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu` (native Ubuntu 24.04 runners, `--locked`), publishes `vxota-<version>-<target>.tar.gz` archives with a `SHA256SUMS`, and attaches build provenance (`gh attestation verify`). `workflow_dispatch` is a dry run that creates no release.
- Open-source release preparation: `LICENSE` (Apache-2.0), `NOTICE`, rewritten `README.md`. Crate `license` metadata is now `Apache-2.0` (was `MIT OR Apache-2.0`).

### Security

- Lockfile update `rustls` 0.23.41 -> 0.23.46, `rustls-webpki` 0.103.13 -> 0.103.16, `webpki-roots` 1.0.8 -> 1.0.10, which fixes RUSTSEC-2026-0285, -0346, -0348, -0349 and -0381 in `vxota-probe`'s TLS stack.

## [0.1.0] - 2026-07-13

**Phase-1 gate passed**: H1, H3, and H4 all have reproducible numbers (`vxperiments`) —
vxota's H4 co-location campaign among them. Repositories were still private at this release.

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
- `vxota-probe fetch --repeat N` reused the same `--out` path across iterations, so
  `download_resumable`'s "already complete" short-circuit made every iteration after the
  first a near-instant no-op digest check instead of a real transfer — H4 driver mode was
  silently measuring almost none of the OTA workload's actual footprint. Now clears the
  output path before each iteration after the first, verified by timing (2026-07-12).

### Known gaps
- Container image (digest-tagged, non-root) + `kind: container` service descriptor
  example; CI publish to GHCR.
