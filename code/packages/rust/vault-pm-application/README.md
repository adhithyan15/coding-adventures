# `coding_adventures_vault_pm_application`

The host-neutral VLT-PM05 application core for the local-first password
manager. The current slices define lossless canonical persistence for local
device secrets, item revisions, and catalog snapshots, plus domain-separated
V1 encrypted object framing and an authority-anchored repository verifier for
the single authorized Phase 1A device.

The crate accepts key and randomness material from its caller. It does not own
a filesystem path, provider SDK, network client, process, environment, clock,
credential store, or entropy source. Bootstrap stores, repository sessions,
crash-recovery journals, and user workflows land in the next slices. There is
no unchecked repository verification path: construction decrypts and
authority-verifies the exact locally pinned certificate frame and object ID,
and commits and announcements must match that vault, device, certificate ID,
and Ed25519 key.

## Verification

The package has 22 tests covering exact canonical and cryptographic vectors,
lossless removed-value persistence, live and tombstone revisions, catalog
bounds and ordering, cross-kind and cross-vault rejection, AEAD tampering,
authority/device/certificate binding, Ed25519 signature rejection, closed
parser failures, diagnostic redaction, and explicit key wiping.
Tarpaulin's LLVM engine measures 547 of 559 production lines covered (97.85%).

## Development

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_application --all-targets -- -D warnings
```
