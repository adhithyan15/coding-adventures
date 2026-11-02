# x509-rsa-constructed-path-signature-routing

Pure public-data composition that authenticates one exact adjacent pair from a
constructed X.509 path with the repository-owned, data-driven RSA signature
router.

The caller supplies:

- one non-copyable adjacency projected from a constructed path;
- certificate-bound algorithm policy for the projected child;
- certificate-wide issuing-CA role policy for the projected issuer;
- certificate-bound RSA algorithm and key policy for that same issuer;
- a profile-index routing table; and
- the reviewed RSA public-operation limit.

Every evidence object must retain the exact decoded certificate projected by
the path. Byte-equal independent decodes are rejected, and a non-CA issuer
fails before signature verification. Success retains the path projection,
role evidence, both algorithm-policy inputs, validated issuer key, and exact
fixed-verifier result.

This package authenticates one projected link only. It does not define full
Name comparison, authenticate another link, apply path length or name
constraints, establish a trust anchor, validate target identity or time, check
revocation, perform TLS or HTTP, authorize OAuth transport, or open a network
connection. It does not make concrete HTTPS or live-provider use ready.

## Validation

```bash
bash BUILD
cargo clippy -p x509-rsa-constructed-path-signature-routing --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-rsa-constructed-path-signature-routing --no-deps
```
