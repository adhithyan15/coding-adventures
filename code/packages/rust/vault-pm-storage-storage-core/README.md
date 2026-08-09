# `coding_adventures_vault_pm_storage_storage_core`

Phase 1A's first persistent password-manager storage adapter. It maps VLT-PM02
opaque vault buckets and object IDs onto an injected `storage-core`
`StorageBackend` while preserving immutable object semantics.

The adapter provides:

- restart-stable binding to one opaque vault locator;
- lowercase-hex namespaces and keys with no caller-readable metadata;
- exact immutable create/replay/corruption classification;
- bucket-bound fixed-width pagination cursors;
- revision-checked physical deletion;
- closed, payload-free error translation; and
- the shared VLT-PM02 conformance suite over memory and filesystem backends.

It owns no filesystem path or provider client. `storage-fs` is the first host
composition, while later cloud adapters can implement VLT-PM02 directly.

## Verification

The package has 12 tests, including the 24-check shared conformance suite over
both memory and filesystem backends, filesystem restart binding, create-race
classification, malformed backend responses, closed error mapping, and cursor
scope enforcement. Tarpaulin's LLVM engine measures 219 of 228 production lines
covered (96.05%).

## Development

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_storage_storage_core --all-targets -- -D warnings
```
