# vxota

Part of the **vxedge** research platform.

**Purpose:** OTA server, signing, and artifact store — the H4 (OTA co-location) service.
See [vxedge/docs](https://github.com/vxedge/docs) for the research plan and
`ARCHITECTURE.md` §8 for the design (Uptane-flavored metadata, why aktualizr/hawkBit/Mender
aren't dependencies).

## Crates

- `vxota-common` — content-addressed artifact store, Uptane-flavored signed root/targets
  metadata, Prometheus helpers.
- `vxota` — the server: `keygen` (root key + `root.json`), `publish` (sign + content-address
  an artifact into `targets.json`), `serve` (HTTP delivery with resumable downloads).
- `vxota-probe` — the client: `fetch` (verify the full trust chain, download, verify the
  digest); drives H4 experiment runs in repeat mode.

## Quick start

```sh
cargo build --workspace

# 1. Generate the root key + root.json. The printed public key is the trust
#    anchor every client pins out-of-band.
./target/debug/vxota keygen --artifacts-dir /tmp/ota-store --out-priv /tmp/root.key

# 2. Publish an artifact (content-addressed + signed into targets.json).
./target/debug/vxota publish --artifacts-dir /tmp/ota-store --file ./firmware.bin \
    --name zone-fw --signing-key /tmp/root.key

# 3. Serve it.
./target/debug/vxota serve --artifacts-dir /tmp/ota-store --listen 0.0.0.0:9900

# 4. Fetch + verify from a client (a zone guest, in H4 experiments).
./target/debug/vxota-probe fetch --server http://<host>:9900 --target zone-fw \
    --trust-root <hex-pubkey-from-step-1> --out ./downloaded.bin

# H4 driver mode: repeated fetches over an experiment window, live Prometheus metrics.
./target/debug/vxota-probe fetch --server http://<host>:9900 --target zone-fw \
    --trust-root <hex-pubkey> --out ./downloaded.bin \
    --repeat 30 --interval-s 10 --metrics 0.0.0.0:9901
```

`cargo test --workspace` runs the full trust-chain end-to-end over loopback (keygen →
publish → serve → fetch/verify), including rejection of a wrong trust-root key and an
unpublished target — see `crates/vxota-probe/tests/loopback.rs`.
