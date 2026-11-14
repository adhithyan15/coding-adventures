# X.509 reviewed server-path anchor self-signature binding

This repository-owned Rust package joins two independently produced evidence
boundaries:

- the strongest accumulated Name-policy-backed reviewed RSA server path; and
- the complete data-driven reviewed-anchor RSA self-signature proof.

It succeeds only when both values retain the same exact decoded reviewed-root
certificate object. Byte equality is intentionally insufficient because a
separate decode must not let evidence for one object authorize another.

```text
reviewed server path ── reviewed terminal ──┐
                                           ├── exact decoded root
reviewed self-signature ── reviewed root ───┘
```

The returned non-copyable value retains both complete inputs. Callers can
inspect them through `reviewed_server_path()` and `anchor_self_signature()`;
they cannot detach the proof after the identity check succeeds.

## Security boundary

This package performs one pointer-identity comparison over public certificate
evidence. It does not parse certificates, route or verify a new signature,
decide trust-anchor semantics, complete RFC 5280 Name/path validation, check
revocation, build TLS or HTTP, access credentials, emit provider/trace audit
events, perform I/O, or authorize an external effect. Concrete HTTPS, OAuth
transport, and live-provider readiness remain unavailable.

All production dependencies are repository-owned. The selected reviewed root
originates in the separately pinned and reviewed `webpki-roots` snapshot; this
package does not turn that data snapshot into a TLS or platform-trust library.

## Development

From `code/packages/rust` with the repository's pinned Rust toolchain:

```bash
cargo test -p x509-rsa-reviewed-server-path-anchor-self-signature-binding
cargo clippy -p x509-rsa-reviewed-server-path-anchor-self-signature-binding --all-targets -- -D warnings
```
