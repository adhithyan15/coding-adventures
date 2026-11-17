# tls13-server-hello-retry-selection

`tls13-server-hello-retry-selection` binds one retained TLS 1.3
HelloRetryRequest selection proof to one retained final ServerHello selection
proof.

It can additionally extract the first ClientHello `supported_groups` and
`key_share`, HelloRetryRequest `key_share`, second ClientHello `key_share`, and
final ServerHello `key_share` directly from those retained messages. One copied
data-driven named-group policy governs every decode. The resulting non-copyable
evidence keeps the retry transition and final server share attached to the
four exact message bodies.

The binder:

- requires HelloRetryRequest first and ordinary ServerHello second,
- requires the retained ClientHello random and legacy session ID to remain
  equal,
- compares the complete cipher-suite and supported-version offers, preserving
  count, order, every identifier, and the data-driven suite policy,
- requires the exact extension-context rule table to remain equal,
- requires the complete policy-authorized cipher-suite descriptor and selected
  TLS version to remain equal, and
- retains both non-copyable proofs for later exact-evidence composition.

The key-share composer:

- locates extension types 10 and 51 only in the already-authorized retained
  extension blocks,
- applies caller-selected list ceilings and one retained named-group policy,
- requires the repository-owned retry-transition rules, and
- decodes the final ServerHello share against the exact retained replacement
  ClientHello offer without accepting a substitute policy.

Failures and diagnostics disclose none of the compared values. The crate does
not validate every permitted ClientHello2 change, generate or prove fresh key
material, invoke key agreement, reject a second retry in connection state,
apply the transcript `message_hash` rewrite, derive traffic keys, perform I/O,
or provide TLS or HTTPS.

## Example

```rust,ignore
let retry_pair = bind_server_hello_retry_selection(retry, server_hello)?;
assert_eq!(retry_pair.retry().version().wire_id(), 0x0304);
# Ok::<(), tls13_server_hello_retry_selection::ServerHelloRetrySelectionError>(())
```

Both inputs are repository-owned non-copyable proofs returned by
`tls13-server-hello-version-selection`.

The separate `bind_retry_key_share_messages` and
`bind_final_server_key_share_message` steps consume that pair, retain the
named-group policy, and return message-bound public key-share evidence. They do
not perform key agreement or advance a handshake.

## References

- [RFC 9846 section 4.2.4](https://www.rfc-editor.org/rfc/rfc9846.html#section-4.2.4)
  requires the final ServerHello to retain the retry cipher suite and selected
  version.
- [RFC 9846 section 4.2.3](https://www.rfc-editor.org/rfc/rfc9846.html#section-4.2.3)
  defines the shared ServerHello and HelloRetryRequest wire structure.
- [RFC 9846 section 4.3.8](https://www.rfc-editor.org/rfc/rfc9846.html#section-4.3.8)
  defines the retry and final `key_share` transitions.

## Development

```bash
cargo test -p tls13-server-hello-retry-selection
cargo clippy -p tls13-server-hello-retry-selection --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-server-hello-retry-selection --no-deps
```
