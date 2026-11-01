# tls13-key-schedule

`tls13-key-schedule` is a narrow repository-owned TLS 1.3 prerequisite. It
composes the no-PSK RFC 8446 base-secret schedule for a full ephemeral key
exchange:

```text
Early Secret     = HKDF-Extract(0, 0)
derived          = Derive-Secret(Early Secret, "derived", "")
Handshake Secret = HKDF-Extract(derived, shared secret)
derived          = Derive-Secret(Handshake Secret, "derived", "")
Master Secret    = HKDF-Extract(derived, 0)
```

The caller's `SelectedCipherSuite` is the sole source of SHA-256 or SHA-384
policy. Canonical empty transcript hashes and the `"derived"` labels are fixed
inside the package. A non-empty caller-owned shared secret is borrowed within a
closed bound; the package neither creates nor validates the selected group's
exact encoding nor authenticates the peer that contributed it.

## Secret ownership

Intermediate derived values use fixed wipe-on-drop storage. The three result
secrets are copied into non-cloneable caller-owned wipe-on-drop storage only
after the complete schedule succeeds. A successful call returns a borrowed
view that retains the selected suite and exposes exact Hash-length secret
prefixes for adjacent repository-owned derivation boundaries. Failed
re-derivation clears all older output first. Debug and error text contain no
secret bytes or caller-supplied lengths.

## Boundary

This package intentionally excludes PSKs, binders, 0-RTT, traffic secrets,
exporter and resumption secrets, transcript sequencing, Finished, key exchange,
AEAD, record protection, handshake processing, certificates, trust roots,
revocation, sockets, HTTP, and OAuth. It performs no external effect and does
not make TLS or HTTPS available.

## Example

```rust
use tls13_cipher_suite_policy::{
    CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry,
};
use tls13_key_schedule::{derive_ephemeral_base_secrets, BaseSecretStorage};

let entry = CipherSuitePolicyEntry::new(
    CipherSuite::TlsAes128GcmSha256,
    1 << 20,
)?;
let selected = CipherSuitePolicy::new(&[entry])?.select(0x1301)?;
let mut storage = BaseSecretStorage::new(selected)?;
let secrets = derive_ephemeral_base_secrets(&[0x42; 32], &mut storage)?;

assert_eq!(secrets.handshake_secret().len(), 32);
assert_eq!(secrets.master_secret().len(), 32);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Development

From `code/packages/rust`:

```bash
cargo test -p tls13-key-schedule
cargo clippy -p tls13-key-schedule --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-key-schedule --no-deps
```
