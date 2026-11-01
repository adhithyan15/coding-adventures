# tls13-handshake

`tls13-handshake` is the allocation-free handshake-message framing layer for a
future repository-owned TLS 1.3 client. It validates RFC 8446's four-byte
message envelope and borrows the exact encoded message without parsing its
body or performing cryptography.

## Why this package exists

Every TLS handshake message starts with an attacker-controlled type and
24-bit body length:

```text
+--------------+---------------------------+
| message type |        body length        |
|    1 byte    |          3 bytes          |
+--------------+---------------------------+
```

That length may span record boundaries, and the complete header plus body is
also the input to the TLS transcript hash. Keeping framing separate makes the
work bound, stream behavior, and future transcript handoff reviewable without
confusing structural recognition with protocol-state authorization.

## Boundary

- Policy is explicit data: a non-empty, duplicate-free allowlist of recognized
  core TLS 1.3 wire message types and a body ceiling no larger than `2^24 - 1`.
- The zero-length `EndOfEarlyData` body remains representable; body-specific
  semantics belong to later parsers and state policy.
- Exact decoders reject trailing bytes. Prefix decoders return the untouched
  stream tail.
- A decoded value borrows both its body and the exact header-plus-body bytes a
  later transcript owner must hash.
- Encoders validate type policy, length, and output capacity before writing, so
  an error leaves caller storage unchanged.
- Synthetic transcript-only `message_hash` is not accepted as a wire message.

The package does not reassemble messages across records, sequence handshake
states, own a transcript hash, parse handshake bodies, negotiate parameters,
derive keys, protect records, validate certificates, establish trust, check
revocation, perform I/O, or grant HTTP, OAuth, or network authority. It does
not make concrete HTTPS available.

## Example

```rust
use tls13_handshake::{
    decode_tls13_handshake, encode_tls13_handshake, Tls13HandshakePolicy,
    Tls13HandshakeType,
};

let policy = Tls13HandshakePolicy::new(&[Tls13HandshakeType::ClientHello], 1024)?;
let mut bytes = [0_u8; 64];
let used = encode_tls13_handshake(
    &policy,
    Tls13HandshakeType::ClientHello,
    &[3, 3],
    &mut bytes,
)?;
let message = decode_tls13_handshake(&policy, &bytes[..used])?;
assert_eq!(message.body(), &[3, 3]);
assert_eq!(message.encoded(), &bytes[..used]);
# Ok::<(), tls13_handshake::HandshakeError>(())
```

## Verification

```bash
bash BUILD
cargo clippy -p tls13-handshake --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-handshake --no-deps
```
