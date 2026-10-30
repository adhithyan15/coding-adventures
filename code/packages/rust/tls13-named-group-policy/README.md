# tls13-named-group-policy

`tls13-named-group-policy` is an allocation-free TLS 1.3 named-group registry
and caller allowlist.

The initial registry recognizes X25519 (`0x001d`) and secp256r1 (`0x0017`). It
keeps each wire ID, key-agreement routing identifier, exact key-exchange length,
and exact shared-secret length together in one immutable descriptor. A caller
constructs a non-empty, duplicate-free allowlist. Selection distinguishes an
unknown wire identifier from a recognized group excluded by that allowlist.

These values are policy metadata only. The package does **not** parse key-share
entries, choose client offers, generate ephemeral secrets, invoke or approve a
repository X25519 or P-256 implementation, validate peer public keys, detect an
all-zero agreement result, derive or retain secrets, process a handshake, or
make TLS or HTTPS available.

## Example

```rust
use tls13_named_group_policy::{
    KeyAgreementAlgorithm, NamedGroup, NamedGroupPolicy,
};

let policy = NamedGroupPolicy::new(&[NamedGroup::X25519])?;
let selected = policy.select(0x001d)?;
let parameters = selected.parameters();

assert_eq!(parameters.group(), NamedGroup::X25519);
assert_eq!(parameters.key_agreement(), KeyAgreementAlgorithm::X25519);
assert_eq!(parameters.key_exchange_length(), 32);
assert_eq!(parameters.shared_secret_length(), 32);
selected.validate_key_exchange_length(32)?;
selected.validate_shared_secret_length(32)?;
# Ok::<(), tls13_named_group_policy::PolicyError>(())
```

## References

- [RFC 8446 section 4.2.7](https://www.rfc-editor.org/rfc/rfc8446.html#section-4.2.7)
  defines the supported-groups negotiation role.
- [RFC 8446 section 4.2.8.2](https://www.rfc-editor.org/rfc/rfc8446.html#section-4.2.8.2)
  defines the secp256r1 and X25519 key-share representations.
- [RFC 8446 section 7.4.2](https://www.rfc-editor.org/rfc/rfc8446.html#section-7.4.2)
  defines their fixed-length shared-secret representations.

## Development

```bash
cargo test -p tls13-named-group-policy
cargo clippy -p tls13-named-group-policy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-named-group-policy --no-deps
```
