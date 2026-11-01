# tls13-record-protection-state

`tls13-record-protection-state` binds one already-derived TLS 1.3 traffic key
and static IV to the matching monotonic record sequence for one direction. It
consumes the success-only view returned by `tls13-traffic-key-material`, creates
nonce policy only from that same policy-selected cipher suite, and returns one
borrowed reservation containing the matching key, nonce, AEAD routing metadata,
authentication-tag length, and consumed sequence number.

It does not make TLS or HTTPS available. It does not derive a traffic secret,
select or negotiate a cipher suite, invoke an AEAD, frame or protect records,
perform key updates, process a handshake, validate certificates, select trust
roots, check revocation, open a socket, or authorize HTTP or OAuth transport.

## Boundary

The state accepts no detached static IV, nonce length, record ceiling, key, or
AEAD identifier. All of those values remain reachable only through one
non-cloneable `TrafficKeyMaterial` success view, and nonce policy is constructed
internally from that view's selected suite. Fresh state begins at sequence zero
and deliberately has no reset operation. A key update must derive fresh key
material and create a new state.

Every successful reservation consumes its sequence before it returns. The
nonce lives in fixed state-owned storage, and the reservation borrows the whole
state while exposing the matching key and public routing metadata. That borrow
prevents another sequence reservation until the current one is released. If a
future AEAD operation fails, its sequence is still consumed and cannot be
silently reused. Exhaustion is fail-closed and leaves state unchanged.

Secret key and static-IV bytes remain in the caller-owned wipe-on-drop storage
borrowed by the traffic-key view. Neither state nor reservation is cloneable,
and their diagnostics redact key material. The composition allocates nothing
and performs no external effect.

## Development

From `code/packages/rust`:

```bash
cargo test -p tls13-record-protection-state
cargo clippy -p tls13-record-protection-state --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-record-protection-state --no-deps
```
