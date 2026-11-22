# tls13-server-hello-version-selection

`tls13-server-hello-version-selection` binds the mandatory TLS 1.3
`supported_versions` selection and extension-context policy to the exact
ClientHello/ServerHello evidence already retained by
`tls13-server-hello-selection`.

The binder:

- validates the retained ClientHello extension block under a caller-selected,
  data-driven policy,
- validates the retained ordinary ServerHello or HelloRetryRequest block in
  its exact context and rejects unsolicited response extensions,
- requires `supported_versions` in both exact messages,
- compares the complete ClientHello extension data with the supplied version
  offer, including vector length, order, and every identifier,
- validates the exact server extension data against that proven offer and
  admits only TLS 1.3, and
- retains the non-copyable prior selection proof, offer, policy, and selected
  version for later exact-evidence composition.

It does not compare retry and final messages, require retry progress, rewrite a
transcript, interpret key shares, derive keys, protect records, perform I/O, or
provide TLS or HTTPS.

## Example

```rust,ignore
let policy = ExtensionContextPolicy::new(&STANDARD_TLS13_EXTENSION_RULES)?;
let bound = bind_server_hello_version_selection(selection, version_offer, policy)?;
assert_eq!(bound.version().wire_id(), 0x0304);
# Ok::<(), tls13_server_hello_version_selection::ServerHelloVersionSelectionError>(())
```

The `selection` input is the repository-owned non-copyable proof returned by
`tls13-server-hello-selection`; the offer and policy are caller-owned data.

## References

- [RFC 8446 section 4.1.3](https://www.rfc-editor.org/rfc/rfc8446.html#section-4.1.3)
  requires `supported_versions` in TLS 1.3 ServerHello messages.
- [RFC 8446 section 4.1.4](https://www.rfc-editor.org/rfc/rfc8446.html#section-4.1.4)
  requires it in HelloRetryRequest and limits response extensions.
- [RFC 8446 section 4.2.1](https://www.rfc-editor.org/rfc/rfc8446.html#section-4.2.1)
  defines the client offer and server selection encodings.

## Development

```bash
cargo test -p tls13-server-hello-version-selection
cargo clippy -p tls13-server-hello-version-selection --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-server-hello-version-selection --no-deps
```
