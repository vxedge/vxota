# TASKS.md — vxedge/vxota

**H4 delivered 2026-07-13** (exp-003, vxperiments): H4 HOLDS — see docs/RESULTS.md and
vxperiments/TASKS.md for the campaign result. This closed the phase-1 gate.

## Phase 1 (not on the critical path; H4 needed it by end of phase 2)

- [x] Crate scaffold + CI (2026-07-12): 3-crate workspace (`vxota-common`, `vxota`,
      `vxota-probe`) mirroring vxagents' conventions — workspace `Cargo.toml`, `deny.toml`,
      `rust-toolchain.toml` (1.96.1), `.github/workflows/ci.yml` (build/test/clippy/fmt +
      cargo-deny). `cargo deny check`: advisories/bans/licenses/sources all pass.
- [x] Artifact store layout: content-addressed by SHA256 (2026-07-12) —
      `vxota-common::store::ArtifactStore`, atomic writes (`<root>/artifacts/<sha256>`),
      idempotent re-publish.

## Phase 2 (H4 experiment support)

- [x] OTA server v1 (2026-07-12): `vxota serve` (axum + `tower_http::ServeDir`) serves
      `root.json`/`targets.json`/`artifacts/<sha256>` directly off the store's directory
      layout — Range/If-Range (resumable downloads) handled by `ServeDir`, verified against
      a real curl request. Config is a CLI flag (`--artifacts-dir`, `--listen`) matching
      the manifest's `config.artifacts`/`config.listen` fields 1:1. `vxedge ota up
      --experiment <manifest>` (in `vxedge`/`vxedged`) spawns/manages this process from
      the experiment manifest's `services.ota` block — delivered 2026-07-12, smoke-tested
      live, used successfully by exp-003 (2026-07-13).
- [x] Signing tooling (2026-07-12): Uptane-flavored signed root/targets metadata
      (`vxota-common::metadata` — vocabulary/format only, no aktualizr dependency, per §8).
      `vxota keygen` generates the root key + self-signed `root.json`; `vxota publish`
      content-addresses + signs an artifact into `targets.json`. Client trust chain (in
      `vxota-probe`) pins the root key out-of-band, verifies `root.json` against it, then
      verifies `targets.json` against the key `root.json` itself vouches for — never trusts
      a server-supplied key it wasn't already vouched for. Unit-tested (roundtrip, wrong-key
      rejection, expiry rejection, tamper rejection) + end-to-end loopback-tested (wrong
      trust-root and unknown-target rejection).
- [x] Prometheus exporter (2026-07-12): `vxota-probe`'s `OtaState` (mirrors vxprobe's
      `ProbeState`) — `vxota_deployments_total{status=success|failure}`,
      `vxota_delivery_latency_seconds{quantile=...}`. The server side also exposes
      `/metrics` with a basic request counter.
- [x] Container image (digest-tagged, non-root) — **delivered 2026-08-04**
      (`vxparts/parts/svc/ota/app/vxota/`, VXPN `VXE92-SOTA-ATPS`, full account in
      `vxparts/TASKS.md`). Published from `vxparts`, not this repo — this repo owns the
      source, `vxparts` owns every loadable artifact the platform boots, same split as
      every other part (`docs/ARCHITECTURE.md` §4, fixed the same day: it never actually
      named `vxparts` as the registry before this). Packaging-only Containerfile (private
      repo, can't `git clone` at build time); binary built here and staged into the part.
      This repo also gained `Containerfile.dev` — compiles from source in one step, for
      local iteration; never published, not the same recipe. Both live-verified: real
      `keygen` run inside each built image, confirmed non-root. **Still separate, still
      open**: the daemon doesn't actually consume this image yet — `vxedge-core::services
      ::OtaBackend` still spawns the bare `vxota` binary as a subprocess
      (`--vxota-bin`/`SpawnSpec`), not a container. A `kind: container` service-descriptor
      path (manifest schema + `vxedged` wiring to actually run the published image) is
      unstarted engineering, not implied done by publishing the artifact.
- [ ] CI: automated *publish* to GHCR with SHA256 digest recorded — **partially covered**:
      `vxparts`' own `build-containers` CI job (delivered 2026-08-04, `docs/ROADMAP.md` #18)
      already build-verifies this part automatically on every push that touches it, since
      it now has a `Containerfile` — but that job deliberately never pushes
      (`vxparts/.github/workflows/parts.yml`'s own stated policy: publishing stays a
      manual `workflow_dispatch` act). Actually publishing this specific part to GHCR is
      still a manual, one-time step someone runs.
- [x] Simple OTA client probe to drive H4 runs (2026-07-12): `vxota-probe fetch` — one-shot
      or `--repeat N --interval-s S` for a sustained H4 experiment window; `--metrics <addr>`
      serves live Prometheus during the run; prints a JSON summary to stdout and exits 1 if
      every attempt failed (mirrors vxprobe's exit-1-on-zero-received convention). Resumable
      download: appends via `Range` only when the server actually answers 206 (verified in
      tests), otherwise restarts clean rather than risking silent corruption. Caught and
      fixed a real bug before exp-003 ran: `--repeat` reused the same `--out` path, so the
      resumability short-circuit made every iteration after the first a near-instant no-op
      instead of a real transfer — fixed by clearing the output path each iteration
      (2026-07-12). Used successfully in exp-003 (2026-07-13): ~355 real deliveries/rep,
      zero failures, ~206ms median latency for a 5MB artifact.

**Known v1 simplifications (documented, not oversights):** single ed25519 key covers both
the root and targets roles (real Uptane splits root/targets/snapshot/timestamp across
different key custody); director-repo / campaign semantics are out of scope (see phase 3);
`sha256_file` reads the whole file into memory rather than streaming (fine at this testbed's
artifact sizes); metadata signing uses serde's own deterministic-but-not-RFC-8785 JSON
encoding since only vxota itself ever produces the signed bytes; transport encryption is the
manifest's `ota` slice WireGuard tunnel (§7), so the server deliberately does not add TLS.

## Phase 3

- [ ] Delivery over the vehicle manifest's `ota` slice with campaign semantics
      (batch of zones, staged rollout, rollback signal) — this is where a director-repo
      analog (Uptane's second metadata repo, for fleet-targeting decisions) would land.
- [ ] Audit log of deliveries keyed by experiment ID + manifest checksum
- [ ] UDS/DoIP diagnostic probe: minimal DoIP endpoint in zone guests + diagnostic session
      exercised during OTA runs (production-realistic update flow)
- [ ] Optional: atomic slot/image-swap updates on vxzones guest bundles via **Rugix Ctrl**
      (Rust, memory-safe, secure-by-default verification, native delta updates — chosen
      over RAUC for the Rust-policy fit; RAUC is the fallback if Rugix's narrower track
      record or its gaps, no NAND/UBI, no encrypted updates — both irrelevant to our
      QEMU/libvirt block storage — become a blocker). See ARCHITECTURE.md §8
