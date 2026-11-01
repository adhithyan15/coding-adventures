# tls13-transcript

`tls13-transcript` is the fixed-memory transcript-hash owner for a future
repository-owned TLS 1.3 state machine. It hashes through both SHA-256 and
SHA-384 before cipher-suite selection, then consumes a policy-authorized suite
and retains only that suite's hash state.

## Why both hashes exist before selection

The ClientHello is part of the transcript before the server selects a cipher
suite. The selected suite then fixes the key-schedule hash:

```text
caller-accepted ClientHello encoding
             |
       +-----+------+
       |            |
     SHA-256      SHA-384
       |            |
       +-----+------+
             |
    selected cipher suite
             |
      one retained state
```

Re-parsing or replaying the ClientHello after selection would create a second
source of truth. Keeping both small fixed-memory states until the selected
suite is policy-authorized avoids that ambiguity.

## Boundary

- A caller-selected nonzero cumulative wire-byte ceiling bounds all external
  transcript input before either hash state changes.
- Selection consumes a `SelectedCipherSuite`; the transcript cannot choose a
  detached hash algorithm.
- Snapshots carry the selected SHA-256/SHA-384 identifier and exact digest
  length together, and do not mutate retained state.
- HelloRetryRequest rewriting replaces the current selected state with the
  exact synthetic `message_hash` header and first-ClientHello digest, and may
  happen only once.
- Stable errors contain no handshake bytes, digest bytes, suite identifier, or
  caller length.

Callers must supply exact bytes from handshake messages already accepted by
their framing and state layers, and invoke selection and HelloRetryRequest
rewriting at the correct protocol transitions. This package does not parse or
authenticate framing, sequence handshake messages,
negotiate a suite, derive secrets, protect records, process certificates,
establish trust, enforce revocation, perform I/O, or grant HTTP, OAuth, or
network authority. It does not make concrete HTTPS available.

## Example

```rust
use tls13_cipher_suite_policy::{
    CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry,
};
use tls13_transcript::{PendingTls13Transcript, TranscriptPolicy};

let entry = CipherSuitePolicyEntry::new(CipherSuite::TlsAes128GcmSha256, 1024)?;
let suites = CipherSuitePolicy::new(&[entry])?;
let selected_suite = suites.select(0x1301)?;

let mut pending = PendingTls13Transcript::new(TranscriptPolicy::new(1 << 20)?);
pending.update(b"caller-accepted ClientHello encoding")?;
let selected = pending.select(selected_suite);
assert_eq!(selected.snapshot().as_bytes().len(), 32);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Verification

```bash
bash BUILD
cargo clippy -p tls13-transcript --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-transcript --no-deps
```
