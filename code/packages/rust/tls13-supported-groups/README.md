# tls13-supported-groups

`tls13-supported-groups` is an allocation-free decoder for the exact RFC 8446
section 4.2.7 `NamedGroupList` extension-data shape.

It requires the non-empty, even-length, two-byte-length-prefixed vector to
consume the complete input, applies an explicit caller-selected group-count
ceiling, retains the exact borrowed extension data, and iterates over each
two-byte identifier in preference order. Unknown, private-use, and repeated
identifiers remain wire data for later policy. Stable failures and diagnostics
do not retain or display identifiers or input bytes.

This package does not locate `supported_groups` in a handshake message, apply
extension-context policy, require or forbid the extension, select or authorize
a group, bind a key share to an offer, parse or generate key-share bytes,
invoke key exchange, process a transcript or certificate, establish trust,
enforce revocation, perform I/O, provide HTTP/OAuth transport, or make concrete
HTTPS or live-provider use ready.

## Development

```bash
bash BUILD
cargo clippy -p tls13-supported-groups --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-supported-groups --no-deps
```
