# AGENTS.md — vxedge/vxota

OTA server, signing tooling, and artifact store. This is *service code*: its runtime
descriptor (image + config) for a given experiment lives in that experiment's manifest in
`vxperiments` — the code lives here. Master spec: `../docs/ARCHITECTURE.md` §8 (service
model), §12 (OTA metrics), hypothesis H4 (OTA co-location vs ADAS SLAs).

## What lands here

- OTA server (Rust) — serves signed artifacts to zones/compute over the `ota` slice,
  transported over WireGuard (§7).
- Signing tooling: SHA256 digests + signature verification for all delivered artifacts.
- Artifact store layout + retention conventions.

## Rules

- Deployment is on-demand via `vxedge ota up --experiment <manifest>`; the server must be
  stateless across runs apart from its artifact directory (`config.artifacts`).
- Must export the H4/OTA metrics from day one: deployment success rate, artifact delivery
  latency (Prometheus exporter, scraped by vxtelemetry).
- Container-first packaging (`kind: container`), digest-tagged; no root in the container.
- Rust (§15 policy); MIT/Apache-2.0.

## Project skills

AI/contributor skills in `.claude/skills/`: **testbed-quality** (the OEM-adoption bar —
reproducibility, fidelity honesty, interface stability, standards legibility).
