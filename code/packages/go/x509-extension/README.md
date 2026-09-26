# x509-extension (Go)

`x509-extension` decodes the generic RFC 5280 `Extension` container: an
OBJECT IDENTIFIER, an omitted-default critical flag, and an opaque OCTET
STRING. It shares the caller's `der-asn1` limits and owns no extension
registry, certificate policy, or cryptographic authority.

The native test front door consumes all 48 language-neutral X.509 Extension
v1 cases.

## Dependencies

- der-asn1
- der-tlv (stable framing error categories)
