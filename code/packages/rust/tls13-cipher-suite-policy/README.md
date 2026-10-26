# tls13-cipher-suite-policy

`tls13-cipher-suite-policy` is an allocation-free repository-owned policy
boundary for three TLS 1.3 cipher suites. It binds each exact RFC 8446 wire ID
to its AEAD, HKDF hash, key length, IV length, authentication-tag length, and a
conservative per-traffic-key record ceiling.

It does **not** implement an AEAD or hash, negotiate a suite, derive or store
keys, create nonce state, process handshakes, validate certificates, perform
I/O, or make TLS or HTTPS available.

## Supported metadata

| Wire ID | Suite | Key | IV | Tag | Maximum records per key |
|---------|-------|-----|----|-----|-------------------------|
| `0x1301` | `TLS_AES_128_GCM_SHA256` | 16 | 12 | 16 | `2^24` |
| `0x1302` | `TLS_AES_256_GCM_SHA384` | 32 | 12 | 16 | `2^24` |
| `0x1303` | `TLS_CHACHA20_POLY1305_SHA256` | 32 | 12 | 16 | `u64::MAX` |

The AES-GCM limit is deliberately below RFC 8446 section 5.5's `2^24.5`
full-size-record limit. RFC 8446 states that the TLS record sequence wraps
before ChaCha20-Poly1305 reaches its safety limit; the adjacent nonce package
still stops before an increment could wrap. AES-CCM suites remain unsupported
until their repository-owned primitives and limits are reviewed separately.

## Policy construction

Callers choose a non-empty, duplicate-free allowlist and an explicit nonzero
record limit for each suite. The limit cannot exceed the immutable suite
maximum. Selection accepts one exact wire ID and returns a single value that
keeps all standard parameters and the caller limit together.

```rust
use tls13_cipher_suite_policy::{
    CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry,
};

let allowed = [CipherSuitePolicyEntry::new(
    CipherSuite::TlsAes128GcmSha256,
    1 << 20,
)?];
let policy = CipherSuitePolicy::new(&allowed)?;
let selected = policy.select(0x1301)?;

assert_eq!(selected.parameters().key_length(), 16);
assert_eq!(selected.parameters().iv_length(), 12);
assert_eq!(selected.record_limit(), 1 << 20);
# Ok::<(), tls13_cipher_suite_policy::PolicyError>(())
```

## References

- [RFC 8446 appendix B.4](https://www.rfc-editor.org/rfc/rfc8446.html#appendix-B.4)
  defines the TLS 1.3 suite identifiers and AEAD/hash pairings.
- [RFC 8446 section 5.5](https://www.rfc-editor.org/rfc/rfc8446.html#section-5.5)
  defines the key-usage guidance used for the record ceilings.
- [RFC 5116](https://www.rfc-editor.org/rfc/rfc5116.html) and
  [RFC 8439](https://www.rfc-editor.org/rfc/rfc8439.html) define the AEAD
  key, nonce, and tag sizes represented by the immutable descriptors.

## Development

```bash
cargo test -p tls13-cipher-suite-policy
cargo clippy -p tls13-cipher-suite-policy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-cipher-suite-policy --no-deps
```
