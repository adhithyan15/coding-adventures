# tls13-handshake-reassembly

`tls13-handshake-reassembly` is the allocation-free bridge between authenticated
TLS 1.3 record content and the repository-owned handshake-message framing layer.
It retains at most one message in caller-owned storage and returns a borrowed
`tls13-handshake` message only after the complete declared encoding is present.

## Why this package exists

TLS records and handshake messages have independent boundaries. A four-byte
handshake header, its body, or several complete handshake messages may be split
or coalesced across authenticated record contents. The framing package can
recognize a complete encoding, but a future client state machine needs bounded
retention while those authenticated pieces arrive.

## Boundary

- The caller supplies a `Tls13HandshakePolicy` and mutable storage of at least
  four bytes. Storage capacity is the hard per-message retention bound.
- Each push takes raw bytes under the caller precondition that record
  authenticity and the inner handshake content type were already accepted. The
  reassembler cannot distinguish authenticated from unauthenticated input.
- A push copies only through the end of the first message and reports its exact
  consumed count. Any coalesced tail remains in the caller's input slice.
- The header is checked immediately when complete. Disallowed types, policy
  length violations, and declarations larger than storage fail before body
  bytes are copied.
- Starting another message, explicitly resetting, or dropping the reassembler
  clears the initialized portion of caller storage.
- Errors retain only a stable category; debug output does not retain or print
  message bytes, types, lengths, or bodies.

This package does not decode or authenticate TLS records, sequence handshake
states, update a transcript, parse handshake bodies, derive keys, invoke an
AEAD, validate certificates, establish trust, check revocation, perform I/O, or
grant HTTP, OAuth, or network authority. It does not make concrete HTTPS
available.

## Example

```rust
use tls13_handshake::{Tls13HandshakePolicy, Tls13HandshakeType};
use tls13_handshake_reassembly::{ReassemblyProgress, Tls13HandshakeReassembler};

let policy = Tls13HandshakePolicy::new(&[Tls13HandshakeType::Finished], 32)?;
let mut storage = [0_u8; 36];
let mut reassembler = Tls13HandshakeReassembler::new(policy, &mut storage)?;

assert_eq!(
    reassembler.push_authenticated_record_content(&[20, 0])?,
    ReassemblyProgress::Incomplete { consumed: 2 },
);
let progress = reassembler.push_authenticated_record_content(&[0, 2, 0xaa, 0xbb, 8])?;
match progress {
    ReassemblyProgress::Complete { consumed, message } => {
        assert_eq!(consumed, 4);
        assert_eq!(message.body(), &[0xaa, 0xbb]);
    }
    ReassemblyProgress::Incomplete { .. } => unreachable!(),
}
# Ok::<(), Box<dyn std::error::Error>>(())
```
