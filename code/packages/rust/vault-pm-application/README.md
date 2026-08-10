# `coding_adventures_vault_pm_application`

The host-neutral VLT-PM05 application core for the local-first password
manager. The current slices define lossless canonical persistence for local
device secrets, item revisions, and catalog snapshots, plus domain-separated
V1 encrypted object framing and an authority-anchored repository verifier for
the single authorized Phase 1A device. It also defines the exact canonical
`PreparedInit -> Active -> PendingPublication -> Active` owner-state machine,
retry-stable publication journals, encrypted local-secret custody, and
byte-oriented bootstrap/local-state store contracts.

The crate also supplies the production object-safe application repository
factory over an injected VLT-PM02 store. It derives no address itself, requires
an unlocked authority-anchored verifier at connection time, consumes exact
publication batches by value, and translates storage and integrity failures to
a closed payload-free application error surface.

The crate accepts key and randomness material from its caller. It does not own
a filesystem path, provider SDK, network client, process, environment, clock,
credential store, or entropy source. Generation-zero preparation and session
workflows land in the next slices. There is no unchecked repository
verification path: construction decrypts and
authority-verifies the exact locally pinned certificate frame and object ID,
and commits and announcements must match that vault, device, certificate ID,
and Ed25519 key.

## Verification

The package has 34 tests covering exact canonical and cryptographic vectors,
lossless removed-value persistence, live and tombstone revisions, catalog
bounds and ordering, cross-kind and cross-vault rejection, AEAD tampering,
authority/device/certificate binding, Ed25519 signature rejection, closed
parser failures, crash-state relationship checks, diagnostic redaction, and
explicit key wiping. Repository tests additionally exercise initialization,
publication, verified open/read/history, every injected provider operation,
constructor paths, and complete error translation. The exact Tarpaulin result
under its LLVM engine is 1,020 of 1,052 production lines (96.96%).

## Development

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_application --all-targets -- -D warnings
```
