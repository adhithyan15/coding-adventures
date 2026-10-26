# tls13-record-nonce

`tls13-record-nonce` is a small repository-owned prerequisite for the TLS 1.3
record-protection boundary. It owns the sequence counter and nonce construction
defined by RFC 8446 section 5.3 for one traffic key in one direction.

It does not make HTTPS or TLS available. In particular, it does not select or
invoke an AEAD, hold traffic keys or static IVs, encrypt, authenticate, decrypt,
derive secrets, negotiate a cipher suite, update keys, validate certificates,
open sockets, or send HTTP.

## Boundary

The caller supplies a `NoncePolicy` containing:

- the exact static-IV length, from 8 through the package work bound of 255
  bytes; and
- a nonzero per-key record ceiling selected for the negotiated AEAD.

The 255-byte ceiling is not an algorithm registry. The caller must still choose
the IV length permitted by its negotiated AEAD and a record ceiling no larger
than that AEAD's cryptographic usage limit.

`RecordNonceSequence` starts at zero and is intentionally not clonable. It has
no reset operation. Callers create separate values for reading and writing and
replace the value whenever the traffic key changes. A successful reservation
consumes the sequence before returning, so later authentication failure cannot
cause accidental nonce reuse.

The largest accepted record ceiling is `u64::MAX`, which permits sequence
values zero through `u64::MAX - 1`. This conservatively stops before advancing
would wrap; real TLS cipher-suite limits are lower.

## Nonce construction

For each reservation, the crate:

1. validates the borrowed static IV and caller-owned output against the exact
   policy length;
2. validates that the per-key record ceiling has not been reached;
3. encodes the current 64-bit sequence in network byte order;
4. pads that encoding on the left with zeros to the selected IV length;
5. XORs it with the borrowed static IV into caller-owned output; and
6. advances the sequence.

Every validation failure leaves both the output and the sequence unchanged.
Stable errors contain only categories, never IV bytes, nonce bytes, sequence
values, configured limits, or caller lengths. The crate allocates no storage
and has no normal dependencies.

## Example

```rust
use tls13_record_nonce::{NoncePolicy, RecordNonceSequence};

let policy = NoncePolicy::new(12, 2)?;
let mut sequence = RecordNonceSequence::new(policy);
let static_iv = [0xa5; 12];
let mut nonce = [0u8; 12];

let first = sequence.reserve_next_nonce(&static_iv, &mut nonce)?;
assert_eq!(first.sequence_number(), 0);
assert_eq!(nonce, static_iv);

let second = sequence.reserve_next_nonce(&static_iv, &mut nonce)?;
assert_eq!(second.sequence_number(), 1);
assert_eq!(nonce[11], 0xa4);
# Ok::<(), tls13_record_nonce::Error>(())
```

## Development

```bash
cargo test -p tls13-record-nonce
cargo clippy -p tls13-record-nonce --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-record-nonce --no-deps
```
