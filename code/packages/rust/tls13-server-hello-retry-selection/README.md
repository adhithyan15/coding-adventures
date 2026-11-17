# tls13-server-hello-retry-selection

`tls13-server-hello-retry-selection` binds one retained TLS 1.3
HelloRetryRequest selection proof to one retained final ServerHello selection
proof.

The binder:

- requires HelloRetryRequest first and ordinary ServerHello second,
- requires the retained ClientHello random and legacy session ID to remain
  equal,
- compares the complete cipher-suite and supported-version offers, preserving
  count, order, every identifier, and the data-driven suite policy,
- requires the complete policy-authorized cipher-suite descriptor and selected
  TLS version to remain equal, and
- retains both non-copyable proofs for later exact-evidence composition.

Failures and diagnostics disclose none of the compared values. The crate does
not validate every permitted ClientHello2 change, require retry progress,
interpret retry extensions, reject a second retry in connection state, apply
the transcript `message_hash` rewrite, derive keys, perform I/O, or provide TLS
or HTTPS.

## Example

```rust,ignore
let retry_pair = bind_server_hello_retry_selection(retry, server_hello)?;
assert_eq!(retry_pair.retry().version().wire_id(), 0x0304);
# Ok::<(), tls13_server_hello_retry_selection::ServerHelloRetrySelectionError>(())
```

Both inputs are repository-owned non-copyable proofs returned by
`tls13-server-hello-version-selection`.

## References

- [RFC 9846 section 4.2.4](https://www.rfc-editor.org/rfc/rfc9846.html#section-4.2.4)
  requires the final ServerHello to retain the retry cipher suite and selected
  version.
- [RFC 9846 section 4.2.3](https://www.rfc-editor.org/rfc/rfc9846.html#section-4.2.3)
  defines the shared ServerHello and HelloRetryRequest wire structure.

## Development

```bash
cargo test -p tls13-server-hello-retry-selection
cargo clippy -p tls13-server-hello-retry-selection --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-server-hello-retry-selection --no-deps
```
