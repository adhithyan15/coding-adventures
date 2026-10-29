# tls13-finished

`tls13-finished` owns the small cryptographic operation behind a TLS 1.3
Finished message. It derives the fixed `"finished"` key from an
already-derived BaseKey, then generates or verifies
`HMAC(finished_key, Transcript-Hash(Handshake Context))`.

## Boundary

- A policy-authorized `SelectedCipherSuite` is the source of the SHA-256 or
  SHA-384 choice and exact 32-byte or 48-byte lengths.
- An explicit client/server sender role remains bound to key storage and
  successful peer-verification evidence for later state-machine consumption.
- The BaseKey arrives in a fixed wipe-on-drop `HkdfPrk`; its algorithm must
  match the selected suite before derivation begins.
- The finished key occupies caller-owned, non-cloneable fixed wipe-on-drop
  storage and is exposed only through a success-only borrowed view. Secret
  bytes never cross the ABI return slot by value.
- Generation writes verify-data into exact caller-owned storage only after all
  public checks and HMAC computation succeed.
- Verification checks exact public lengths, computes the expected value in
  wipe-on-drop storage, and uses the repository-owned compiler-preserved
  full scan. Success returns non-cloneable evidence retaining the exact public
  transcript hash and selected suite.

The caller owns transcript construction, handshake state transitions, peer
role, and the decision about which BaseKey applies. This package does not
derive traffic secrets, parse or sequence handshake messages, authenticate a
certificate, establish trust, enforce revocation, protect records, perform
I/O, or authorize HTTP or OAuth transport. It does not make concrete HTTPS
available.

## Example

```rust
use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk};
use tls13_cipher_suite_policy::{
    CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry,
};
use tls13_finished::{derive_finished_key, FinishedKeyStorage, FinishedRole};

let allowed = [CipherSuitePolicyEntry::new(
    CipherSuite::TlsAes128GcmSha256,
    1 << 20,
)?];
let selected = CipherSuitePolicy::new(&allowed)?.select(0x1301)?;
let base_key = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &[0x42; 32])?;
let mut storage = FinishedKeyStorage::new(selected, FinishedRole::Server);
let finished_key = derive_finished_key(&base_key, &mut storage)?;
let transcript_hash = [0x24; 32];
let mut verify_data = [0u8; 32];

finished_key.write_verify_data(&transcript_hash, &mut verify_data)?;
let verified = finished_key.verify_peer(&transcript_hash, &verify_data)?;
assert_eq!(verified.selected_suite(), selected);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Development

From `code/packages/rust`:

```bash
cargo test -p tls13-finished
cargo clippy -p tls13-finished --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-finished --no-deps
```
