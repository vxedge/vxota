# TASKS.md — vxedge/vxota

## Phase 1 (not on the critical path; H4 needs it by end of phase 2)

- [ ] Crate scaffold + CI (build/clippy/fmt/test)
- [ ] Artifact store layout: content-addressed by SHA256, manifest of artifacts

## Phase 2 (H4 experiment support)

- [ ] OTA server v1: HTTP(S) artifact delivery, resumable downloads, config via service
      `config:` block (`artifacts:` dir)
- [ ] Signing tooling: sign on publish, verify on client fetch (reject unsigned/mismatched);
      Uptane-flavored metadata shape (signed root/targets-style manifest, image-repo vs.
      director-repo separation) — vocabulary/format only, no aktualizr dependency (§8)
- [ ] Prometheus exporter: deployment success rate, delivery latency histogram
- [ ] Container image (digest-tagged, non-root) + `kind: container` service descriptor example
- [ ] CI: build + publish image to GHCR with SHA256 digest recorded (release/artifact pipeline)
- [ ] Simple OTA client probe (for zone guests) to drive H4 runs

## Phase 3

- [ ] Delivery over the vehicle manifest's `ota` slice with campaign semantics
      (batch of zones, staged rollout, rollback signal)
- [ ] Audit log of deliveries keyed by experiment ID + manifest checksum
- [ ] UDS/DoIP diagnostic probe: minimal DoIP endpoint in zone guests + diagnostic session
      exercised during OTA runs (production-realistic update flow)
- [ ] Optional: atomic slot/image-swap updates on vxzones guest bundles via **Rugix Ctrl**
      (Rust, memory-safe, secure-by-default verification, native delta updates — chosen
      over RAUC for the Rust-policy fit; RAUC is the fallback if Rugix's narrower track
      record or its gaps, no NAND/UBI, no encrypted updates — both irrelevant to our
      QEMU/libvirt block storage — become a blocker). See ARCHITECTURE.md §8
