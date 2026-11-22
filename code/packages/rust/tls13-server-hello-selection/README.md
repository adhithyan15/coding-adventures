# tls13-server-hello-selection

`tls13-server-hello-selection` binds a structurally decoded TLS 1.3
`ServerHello` or `HelloRetryRequest` to the exact decoded `ClientHello` and
one supplied policy-bearing cipher-suite offer.

The binder:

- compares the complete policy offer with the decoded ClientHello
  cipher-suite vector, including count, values, and order,
- rejects detached or substituted offer evidence before using its policy,
- requires the exact legacy session ID echo,
- requires the server-selected suite to occur in the original offer,
- reuses the same data-driven cipher-suite policy to authorize that suite, and
- retains both non-copyable decoded messages and the offer so later layers
  cannot substitute different evidence after binding, and
- returns the ordinary-or-retry classification plus the selected algorithm,
  HKDF hash, key-length, and record-limit descriptor.

The package does not parse handshake bodies, interpret extensions, validate
`supported_versions`, compare retry and final selections, update the
transcript, sequence a handshake, derive keys, protect records, perform I/O,
or provide TLS or HTTPS.

## Example

```rust,ignore
let bound = bind_server_hello_selection(client_hello, client_offer, server_hello)?;
assert_eq!(bound.kind(), Tls13ServerHelloKind::ServerHello);
let parameters = bound.cipher_suite().parameters();
# Ok::<(), tls13_server_hello_selection::ServerHelloSelectionError>(())
```

The three inputs in the example are repository-owned validated evidence:
`ClientHelloMessage`, `ClientCipherSuiteOffer`, and `Tls13ServerHello`.

## References

- [RFC 8446 section 4.1.3](https://www.rfc-editor.org/rfc/rfc8446.html#section-4.1.3)
  requires the exact session ID echo and a cipher suite from the ClientHello.
- [RFC 8446 section 4.1.4](https://www.rfc-editor.org/rfc/rfc8446.html#section-4.1.4)
  applies the ServerHello structure to HelloRetryRequest and defines the
  additional retry-to-final invariants left for a later stateful layer.

## Development

```bash
cargo test -p tls13-server-hello-selection
cargo clippy -p tls13-server-hello-selection --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-server-hello-selection --no-deps
```
