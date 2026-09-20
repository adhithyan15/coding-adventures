# x509-subject-alt-name

`x509-subject-alt-name` is the allocation-free RFC 5280 Subject Alternative
Name layer above the repository-owned `x509-extension`, `der-asn1`, and
`der-tlv` primitives. It recognizes only OID `2.5.29.17`, enforces a fixed
256-name work bound, and retains borrowed DNS-ID strings plus typed IPv4/IPv6
addresses for the separate `tls-server-identity` matcher.

## Security boundary

The decoder requires a non-empty `GeneralNames` sequence and exact
context-specific tag/class/form envelopes. It validates all primitive
GeneralName choices: IA5String syntax for rfc822Name, dNSName, and URI; four-
or sixteen-octet IP addresses; and canonical registered-ID object identifiers.
Constructed otherName, X.400, directoryName, and EDI party name choices are
bounded and counted but remain opaque because their schemas belong in separate
reviewable packages. Unknown choices and empty names fail closed. Errors retain
neither source bytes nor names.

Successful decoding does not validate DNS syntax or establish a server
identity. It does not interpret constructed names, parse a complete
certificate, enforce extension criticality, construct or validate paths,
verify signatures, load trust roots, process revocation, perform TLS, or open a
network connection.

## Verification

```bash
bash BUILD
cargo clippy -p x509-subject-alt-name --all-targets -- -D warnings
cargo doc -p x509-subject-alt-name --no-deps
```
