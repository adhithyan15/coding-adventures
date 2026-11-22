# tls13-record-protection

`tls13-record-protection` is the allocation-free dispatcher for the
repository's reviewed TLS 1.3 record-protection routes. It reads the selected
cipher suite only from `tls13-record-protection-state` and routes to either the
repository-owned AES-128-GCM or ChaCha20-Poly1305 composition. There is no
caller-supplied algorithm, key, nonce, tag length, or fallback.

The recognized `TLS_AES_256_GCM_SHA384` suite is intentionally unavailable
until a separate repository-owned fixed-memory AES-256-GCM primitive and
record route are reviewed. That selection fails before inner encoding, outer
record parsing, caller-buffer mutation, or nonce consumption.

Successful opening returns one non-cloneable view with only authenticated
content, protected content type, and padding accessors. The concrete route
remains private, and dropping the view clears the exact opened prefix through
the selected route's reviewed wipe-on-drop owner. Stable errors identify the
route without retaining peer, content, key, nonce, tag, or output bytes.

This crate does not derive traffic secrets, select epochs, run a handshake,
validate certificates, paths, names, trust, or revocation, perform I/O,
authorize HTTP or OAuth, provide a TLS connector, or make concrete HTTPS
available.

## Development

From `code/packages/rust`:

```bash
cargo test -p tls13-record-protection
cargo clippy -p tls13-record-protection --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-record-protection --no-deps
```
