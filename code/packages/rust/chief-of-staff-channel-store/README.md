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

The public `profile` module exposes the production D18S state and D18A cursor
codecs, content types, bounds, and stable D18P error classifier used by the
shared fixture at `code/fixtures/chief-of-staff-channel/v1/manifest.json`.
It delegates to these same store internals rather than maintaining a second
compatibility implementation.

## Validation

```sh
cargo test -p chief-of-staff-channel-store -- --nocapture
cargo clippy -p chief-of-staff-channel-store --all-targets -- -D warnings
```
