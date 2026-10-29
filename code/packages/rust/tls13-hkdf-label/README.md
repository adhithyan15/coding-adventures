# tls13-hkdf-label

`tls13-hkdf-label` validates and serializes the public `HkdfLabel` information
defined by RFC 8446 section 7.1, composes it with the repository-owned
fixed-memory HKDF implementation, and exposes the adjacent `Derive-Secret`
operation over a caller-supplied transcript hash. It performs no heap
allocation and derives only into caller-owned storage.

## Boundary

The crate binds:

- one public 16-bit requested output length;
- a caller label from 1 through 249 bytes;
- the exact six-byte `"tls13 "` namespace prefix; and
- a context from 0 through 255 bytes.

The complete encoding is at most 514 bytes. `hkdf_expand_label_fixed` retains
that encoding in a fixed stack buffer and passes it to `hkdf_expand_fixed`.
The data-driven SHA-256 or SHA-384 selection is carried by the wipe-on-drop
`HkdfPrk`, preventing an Extract/Expand mismatch. Public failures are checked
before output is modified; unexpected PRF failure wipes caller output.

`derive_secret_from_transcript_hash_fixed` accepts only a transcript hash and
output whose lengths equal the PRK-selected `Hash.length`: 32 bytes for SHA-256
or 48 bytes for SHA-384. It then invokes `HKDF-Expand-Label` with the requested
label and derives exactly one Hash-length secret. The caller owns transcript
hashing and sequencing; this boundary cannot accept raw handshake messages.

This crate performs only in-process secret derivation. It performs no
credential access, signing, audit, I/O, or external effect. It does not select
a cipher suite, compose or retain a complete TLS key schedule, manage
transcripts or nonces, protect records, parse or validate certificates, select
roots, enforce revocation, open a network connection, authorize HTTP, or make
concrete HTTPS available.

## Example

```rust
use tls13_hkdf_label::HkdfLabel;

let label = HkdfLabel::new(16, b"key", b"").unwrap();
let mut storage = [0u8; 13];
assert_eq!(
    label.encode(&mut storage).unwrap(),
    b"\x00\x10\x09tls13 key\x00"
);
```

Fixed-memory composition is explicit:

```rust
use coding_adventures_hkdf::{hkdf_extract_fixed, FixedHkdfHash};
use tls13_hkdf_label::hkdf_expand_label_fixed;

let prk = hkdf_extract_fixed(b"salt", b"ikm", FixedHkdfHash::Sha384).unwrap();
let mut output = [0u8; 32];
hkdf_expand_label_fixed(&prk, b"derived", b"transcript hash", &mut output).unwrap();
```

`Derive-Secret` requires an already-computed, algorithm-sized transcript hash:

```rust
use coding_adventures_hkdf::{hkdf_extract_fixed, FixedHkdfHash};
use tls13_hkdf_label::derive_secret_from_transcript_hash_fixed;

let prk = hkdf_extract_fixed(b"salt", b"ikm", FixedHkdfHash::Sha256).unwrap();
let transcript_hash = [0x42; 32];
let mut secret = [0u8; 32];
derive_secret_from_transcript_hash_fixed(
    &prk,
    b"c hs traffic",
    &transcript_hash,
    &mut secret,
)
.unwrap();
```

## Development

From `code/packages/rust`:

```bash
cargo test -p tls13-hkdf-label
cargo clippy -p tls13-hkdf-label --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-hkdf-label --no-deps
```
