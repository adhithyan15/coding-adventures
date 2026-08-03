# chief-of-staff-channel-store

`chief-of-staff-channel-store` persists D18 encrypted channel records through
the repository-owned `StorageBackend` interface.

Its append protocol is deliberately two-phase:

1. CAS-persist the exact authenticated message header as pending and advance
   the durable next sequence.
2. Encrypt only after that reservation succeeds.
3. Idempotently store the encrypted message record.
4. CAS-clear the pending header.

A crash at any step can resume the exact same header and plaintext hash. If a
pending append is deliberately abandoned, its sequence remains consumed, so a
nonce is never reused. The store also provides ordered reads, monotonic
per-receiver acknowledgements, and idempotent sealed key-grant persistence.

The package never stores plaintexts, CMKs, receiver private keys, or unwrapped
grants. Backend choice remains injected by the caller.

## Validation

```sh
cargo test -p chief-of-staff-channel-store -- --nocapture
cargo clippy -p chief-of-staff-channel-store --all-targets -- -D warnings
```
