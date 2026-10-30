# tls13-key-share

Allocation-free TLS 1.3 `key_share` extension framing under the existing
repository-owned named-group policy.

```rust
use tls13_key_share::{encode_client_key_shares, validate_client_key_share};
use tls13_named_group_policy::{NamedGroup, NamedGroupPolicy};

let policy = NamedGroupPolicy::new(&[NamedGroup::X25519]).unwrap();
let public_key = [0x42; 32];
let share = validate_client_key_share(&policy, 0x001d, &public_key).unwrap();
let mut output = [0u8; 38];
let encoded = encode_client_key_shares(&[share], &mut output).unwrap();
assert_eq!(encoded.as_bytes().len(), 38);
```

Client share construction and decoding require an exact policy-authorized
group and exact standard public key-exchange length. Client encoding rejects
empty and duplicate lists and writes only after complete validation. Server
decoding accepts one exact entry and binds it to the exact retained client
offer.

## Boundary

Key-exchange bytes are public handshake data. This package does not implement
HelloRetryRequest or bind `supported_groups`; generate entropy or an ephemeral
private key; invoke X25519 or P-256; validate a peer point beyond exact length;
reject low-order keys; derive or retain a shared secret; update a transcript;
process a handshake; derive traffic secrets; protect records; validate
certificates; establish trust; enforce revocation; perform I/O; authorize HTTP
or OAuth transport; or open a network connection. It does not establish TLS,
HTTPS, or live-provider readiness.

## Development

```bash
bash BUILD
```
