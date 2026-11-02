# tls13-traffic-key-material

`tls13-traffic-key-material` is a small repository-owned TLS 1.3 key-schedule
prerequisite. It binds one policy-selected cipher suite to an already-derived
traffic secret, then derives exactly that suite's traffic key and static IV by
calling the fixed-memory RFC 8446 `HKDF-Expand-Label` boundary with the labels
`"key"` and `"iv"` and an empty context.

It does not make TLS or HTTPS available. It does not negotiate a suite, derive
or retain traffic secrets, compose the early/handshake/master key schedule,
invoke an AEAD, construct per-record nonces, protect records, process a
handshake, validate certificates, select trust roots, check revocation, open a
socket, or authorize HTTP or OAuth transport.

## Boundary

The caller supplies:

- a `SelectedCipherSuite` produced by the caller's explicit cipher-suite
  allowlist; and
- an `HkdfPrk` containing an already-derived traffic secret.

Before deriving either output, the crate requires the PRK's SHA-256 or SHA-384
policy to match the hash fixed by the selected suite. Key and IV lengths come
only from the immutable suite descriptor; callers cannot provide detached
lengths or labels. The selected suite and its caller-chosen record ceiling stay
bound to the resulting owner.

The largest supported key is 32 bytes and every currently admitted suite uses
a 12-byte IV. Both live in caller-owned fixed `Zeroizing` storage. That storage
has no secret accessors; a successful derivation returns a borrowed
`TrafficKeyMaterial` view, so zero or stale storage cannot be used as validated
key material. Neither type is cloneable, and their `Debug` output contains
metadata only. Key and IV derivation use separate wipe-on-drop staging owners;
every exit wipes those original stack slots, and success copies only the exact
prefixes into the caller's storage. No secret-bearing value crosses the ABI
return slot by value. A release-codegen regression gate verifies both staging
scrubs and rejects allocator references in the composition boundary.

## Example

```rust
use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk};
use tls13_cipher_suite_policy::{
    CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry,
};
use tls13_traffic_key_material::{
    derive_traffic_key_material, TrafficKeyMaterialStorage,
};

let allowed = [CipherSuitePolicyEntry::new(
    CipherSuite::TlsAes128GcmSha256,
    1 << 20,
)?];
let selected = CipherSuitePolicy::new(&allowed)?.select(0x1301)?;
let traffic_secret = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &[0x42; 32])?;
let mut storage = TrafficKeyMaterialStorage::new(selected)?;
let material = derive_traffic_key_material(&traffic_secret, &mut storage)?;

assert_eq!(material.key().len(), 16);
assert_eq!(material.iv().len(), 12);
assert_eq!(material.selected_suite(), selected);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Development

From `code/packages/rust`:

```bash
cargo test -p tls13-traffic-key-material
cargo clippy -p tls13-traffic-key-material --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-traffic-key-material --no-deps
```
