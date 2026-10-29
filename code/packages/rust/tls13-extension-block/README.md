# tls13-extension-block

`tls13-extension-block` is a bounded allocation-free decoder for the generic
TLS extension-vector syntax used by TLS 1.3 handshake messages. It validates
one exact two-byte payload-length prefix, decodes fixed-capacity borrowed
extension entries in wire order, rejects duplicate types, and preserves unknown
types and their complete opaque data for later context-specific policy.

It does not make TLS or HTTPS available. It embeds no extension registry,
decodes no extension value, and does not decide which extensions are permitted,
required, or forbidden in ClientHello, ServerHello, EncryptedExtensions, or any
other handshake message. It does not process a handshake, invoke cryptography,
validate certificates, establish trust, check revocation, perform I/O, open a
socket, or authorize HTTP or OAuth transport.

## Boundary

The caller supplies explicit maximum payload bytes and entry count. Payload
length may be zero for handshake contexts whose extension vector may be empty;
the retained entry ceiling must be between one and 64. The decoder requires the
declared two-byte length to cover the entire remaining input, so both truncation
and trailing bytes fail closed. Each entry then requires an exact type, exact
opaque-data length, and complete data. Duplicate types fail before the second
entry is retained.

Unknown extension types are ordinary syntax values at this layer. Success does
not mean an extension is registered, implemented, allowed in the current
message, or semantically valid. Later message-specific packages must apply
those rules and decode selected values under their own limits.

The returned block contains at most 64 borrowed entries in a fixed array. It
allocates nothing, copies no extension data, and exposes opaque bytes only
through an explicit accessor. Diagnostics contain public type/count/length
metadata but never extension-data bytes; failures contain no peer-supplied type,
length, or data values.

## Development

From `code/packages/rust`:

```bash
cargo test -p tls13-extension-block
cargo clippy -p tls13-extension-block --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-extension-block --no-deps
```
