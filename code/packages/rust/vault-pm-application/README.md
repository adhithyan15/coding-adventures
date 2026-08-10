# `coding_adventures_vault_pm_application`

The host-neutral VLT-PM05 application core for the local-first password
manager. This first slice defines lossless canonical persistence for local
device secrets, item revisions, and catalog snapshots, plus domain-separated
V1 encrypted object framing.

The crate accepts key and randomness material from its caller. It does not own
a filesystem path, provider SDK, network client, process, environment, clock,
credential store, or entropy source. Bootstrap, repository sessions,
crash-recovery journals, and user workflows land in the next slices.

## Verification

The package has 17 tests covering exact canonical and cryptographic vectors,
lossless removed-value persistence, live and tombstone revisions, catalog
bounds and ordering, cross-kind and cross-vault rejection, AEAD tampering,
closed parser failures, diagnostic redaction, and explicit key wiping.
Tarpaulin's LLVM engine measures 454 of 466 production lines covered (97.42%).

## Development

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_application --all-targets -- -D warnings
```
