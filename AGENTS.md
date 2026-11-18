# AGENTS.md — vxedge/vxota

OTA server, signing tooling, and artifact store. This is *service code*: its runtime
descriptor (image + config) for a given experiment lives in that experiment's manifest in
`vxperiments` — the code lives here. Master spec: `../docs/ARCHITECTURE.md` §8 (service
model), §12 (OTA metrics), hypothesis H4 (OTA co-location vs ADAS SLAs).

## What is here

Three-crate Rust workspace (delivered 2026-07-12, mirrors vxagents' layout):

- `vxota-common` — content-addressed artifact store (SHA256), Uptane-flavored signed
  root/targets metadata (§8: vocabulary/format only, no aktualizr dependency), Prometheus
  helpers.
- `vxota` — the OTA server (Rust) — `keygen`/`publish`/`serve`. Serves signed artifacts to
  zones/compute over the `ota` slice, transported over WireGuard (§7) — the server
  deliberately does not add its own TLS on top of that tunnel.
- `vxota-probe` — the client probe: fetches + verifies the full trust chain, drives H4
  experiment runs (one-shot or repeated), exports Prometheus deployment-success/latency
  metrics the same way vxprobe exports RTT.

See `README.md` for the CLI quick start. Work items are tracked on the maintainers' internal
board; the public record of changes is `CHANGELOG.md`.

## Rules

- **Branch names use the `vx/` prefix**, for example `vx/fix-probe-drift`. Never name a branch after a
  tool or vendor. Delete a branch once its PR is merged.
- **Merge only after formal checks.** Before a PR is merged: its CI is green on the PR head; `cargo build --workspace`,
  `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`
  and `cargo deny check`
  ran clean locally; and the diff was re-read for claims beyond `docs/FACTS.md`, host details and
  personal or private material. A failing check blocks the merge; never skip, weaken or disable one.
- **Describe vxedge with `project.summary` from `docs/FACTS.md`** (the README's first lines quote it).
  Do not write another description of vxedge; 5G slicing is one family of its measured cases.
- Deployment is on-demand via `vxedge ota up --experiment <manifest>`; the server must be
  stateless across runs apart from its artifact directory (`config.artifacts`).
- Must export the H4/OTA metrics from day one: deployment success rate, artifact delivery
  latency (Prometheus exporter, scraped by vxtelemetry).
- Container-first packaging (`kind: container`), digest-tagged; no root in the container.
- Rust (§15 policy); MIT/Apache-2.0.

## Project skills

AI/contributor skills in `.claude/skills/`: **rust-patterns** (house Rust rules, per the
Microsoft Rust Patterns book) and **testbed-quality** (the OEM-adoption bar — reproducibility,
fidelity honesty, interface stability, standards legibility). Apply both to every change.
