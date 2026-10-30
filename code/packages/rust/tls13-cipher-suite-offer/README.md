# tls13-cipher-suite-offer

`tls13-cipher-suite-offer` is an allocation-free framing and policy-binding
boundary for the TLS 1.3 client's cipher-suite preference vector.

A caller supplies up to 64 exact two-byte wire identifiers in preference
order plus a repository-owned `CipherSuitePolicy`. Construction rejects an
empty or duplicate offer and requires at least one policy-authorized suite.
Explicitly configured unknown identifiers remain data, allowing a separate
reserved-value or GREASE policy without teaching this package new algorithms.

Encoding writes the RFC 8446 two-byte-length-prefixed vector into caller-owned
storage only after checking the complete capacity. Server selection succeeds
only when the exact identifier was offered and the original policy authorizes
it; success returns the existing descriptor that keeps the suite's algorithms,
lengths, and caller-selected per-key record limit together.

The package does **not** construct a `ClientHello`, generate random values,
decide which unknown identifiers to offer, parse a `ServerHello`, select a
suite on behalf of a peer, invoke cryptography, derive keys, process a
handshake, perform I/O, or make TLS or HTTPS available.

## Example

```rust
use tls13_cipher_suite_offer::ClientCipherSuiteOffer;
use tls13_cipher_suite_policy::{
    CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry,
};

let entries = [CipherSuitePolicyEntry::new(
    CipherSuite::TlsAes128GcmSha256,
    1 << 20,
)?];
let policy = CipherSuitePolicy::new(&entries)?;
let configured = [0x0a0a, 0x1301];
let offer = ClientCipherSuiteOffer::new(&configured, policy)?;
let mut encoded = [0_u8; 6];

assert_eq!(offer.encode(&mut encoded)?, 6);
assert_eq!(encoded, [0, 4, 0x0a, 0x0a, 0x13, 0x01]);
assert_eq!(
    offer.validate_server_selection(0x1301)?.parameters().suite(),
    CipherSuite::TlsAes128GcmSha256,
);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## References

- [RFC 8446 section 4.1.2](https://www.rfc-editor.org/rfc/rfc8446.html#section-4.1.2)
  defines the ClientHello cipher-suite vector and preference ordering.
- [RFC 8446 section 4.1.3](https://www.rfc-editor.org/rfc/rfc8446.html#section-4.1.3)
  requires the server-selected suite to come from the client's offered list.

## Development

```bash
cargo test -p tls13-cipher-suite-offer
cargo clippy -p tls13-cipher-suite-offer --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-cipher-suite-offer --no-deps
```
