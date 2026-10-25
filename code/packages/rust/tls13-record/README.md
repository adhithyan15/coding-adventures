# tls13-record

`tls13-record` is the allocation-free outer-record framing layer for a future
repository-owned TLS 1.3 client. It validates and borrows RFC 8446 plaintext or
ciphertext fragments without opening a socket or performing cryptography.

## Why this package exists

A TLS record starts with five attacker-controlled bytes:

```text
+--------------+----------------+------------------+
| content type | legacy version | fragment length  |
|    1 byte    |    2 bytes     |     2 bytes      |
+--------------+----------------+------------------+
```

Those bytes decide how much stream input belongs to one record and which later
protocol parser may receive it. Keeping framing separate makes its bounds and
state assumptions independently reviewable.

## Boundary

- Plaintext policy is explicit data: a non-empty, duplicate-free content-type
  allowlist and a caller ceiling no larger than `2^14`.
- Ciphertext always uses outer `application_data`, is non-empty, and is bounded
  by `2^14 + 256`; its legacy version must be `0x0303`.
- Plaintext decoding retains but does not interpret or reject any legacy
  version value, including an initial ClientHello's recommended `0x0301`.
- Encoders emit `0x0303`; choosing a different initial-ClientHello wire value
  belongs to the later handshake state layer.
- Exact decoders reject trailing bytes. Prefix decoders return the untouched
  stream tail.
- Encoders validate the complete input and output capacity before writing, so
  an error leaves caller storage unchanged.
- Decoded values borrow their fragments and cannot be constructed through a
  public unchecked path.

The package does not parse handshakes or `TLSInnerPlaintext`, sequence protocol
states, negotiate algorithms, derive keys, encrypt, authenticate, use entropy,
read a clock, validate certificates, check revocation, perform I/O, or grant
HTTP, OAuth, or network authority. It does not replace `tls-platform`, and it
does not make concrete HTTPS available.

## Example

```rust
use tls13_record::{
    decode_tls13_plaintext, encode_tls13_plaintext, Tls13PlaintextPolicy,
    TlsContentType,
};

let policy = Tls13PlaintextPolicy::new(&[TlsContentType::Handshake], 1024)?;
let mut bytes = [0_u8; 64];
let used = encode_tls13_plaintext(
    &policy,
    TlsContentType::Handshake,
    &[1, 0, 0, 0],
    &mut bytes,
)?;
let record = decode_tls13_plaintext(&policy, &bytes[..used])?;
assert_eq!(record.fragment(), &[1, 0, 0, 0]);
# Ok::<(), tls13_record::RecordError>(())
```

## Verification

```bash
bash BUILD
cargo clippy -p tls13-record --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-record --no-deps
```
