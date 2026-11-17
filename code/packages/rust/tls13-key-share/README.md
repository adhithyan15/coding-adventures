# tls13-key-share

Allocation-free TLS 1.3 `key_share` extension framing under the existing
repository-owned named-group policy.

```rust
use tls13_key_share::{
    decode_client_key_shares, encode_client_key_shares, validate_client_key_share,
    KeyShareLimits,
};
use tls13_named_group_policy::{NamedGroup, NamedGroupPolicy};

let policy = NamedGroupPolicy::new(&[NamedGroup::X25519]).unwrap();
let public_key = [0x42; 32];
let share = validate_client_key_share(&policy, 0x001d, &public_key).unwrap();
let mut output = [0u8; 38];
let encoded = encode_client_key_shares(&[share], &mut output).unwrap();
assert_eq!(encoded.as_bytes().len(), 38);
let decoded = decode_client_key_shares(
    encoded.as_bytes(),
    &policy,
    KeyShareLimits::new(1).unwrap(),
).unwrap();
assert_eq!(decoded.extension_data(), encoded.as_bytes());
```

Every nonempty client share requires an exact policy-authorized group and exact
standard public key-exchange length. Client construction and decoding also
admit the exact empty `00 00` vector that requests a HelloRetryRequest group
selection round trip. Encoding rejects duplicate groups and writes only after
complete validation. Decoding retains the exact complete borrowed extension
data, including its vector-length prefix, for later direct message-context
comparison. Server decoding accepts one exact entry and binds it to the exact
retained client offer, so an empty offer cannot authorize a direct ServerHello
share.

## Boundary

Key-exchange bytes are public handshake data. Admitting the empty vector is
syntax evidence only. This package does not implement or authorize
HelloRetryRequest, bind `supported_groups`, require a retry to make progress,
generate entropy or an ephemeral private key, invoke X25519 or P-256, validate
a peer point beyond exact length, reject low-order keys, derive or retain a
shared secret, update a transcript, process a handshake, derive traffic
secrets, protect records, validate certificates, establish trust, enforce
revocation, perform I/O, authorize HTTP or OAuth transport, or open a network
connection. It does not establish TLS, HTTPS, or live-provider readiness.

## Development

```bash
bash BUILD
```
