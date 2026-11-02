# tls13-signature-scheme-list

`tls13-signature-scheme-list` is an allocation-free decoder for the exact RFC
8446 section 4.2.3 `SignatureSchemeList` extension-data shape shared by
`signature_algorithms` and `signature_algorithms_cert`.

It requires the non-empty, even-length, two-byte-length-prefixed vector to
consume the complete input, applies an explicit caller-selected scheme-count
ceiling, retains the exact borrowed extension data, and iterates over each
two-byte identifier in preference order. Unknown, private-use, and repeated
identifiers remain wire data for later policy. Stable failures and diagnostics
do not retain or display identifiers or input bytes.

This package does not locate an extension in a handshake message, apply
extension-context policy, require or forbid either extension, select or
authorize a scheme, bind a response to an offer, parse a public key, invoke
cryptography, verify a signature, process a transcript or certificate,
establish trust, enforce revocation, perform I/O, provide HTTP/OAuth transport,
or make concrete HTTPS or live-provider use ready.

## Development

```bash
bash BUILD
cargo clippy -p tls13-signature-scheme-list --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-signature-scheme-list --no-deps
```
