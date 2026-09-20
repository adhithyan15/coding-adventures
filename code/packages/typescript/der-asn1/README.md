# DER ASN.1 for TypeScript

Bounded typed ASN.1 DER value decoding over the language-neutral
`der-asn1-v1` behavior contract. The package consumes all 109 portable cases,
including all 46 referenced DER-TLV framing cases.

## Dependencies

- der-tlv

The typed layer adds canonical primitive decoders, exact constructed wrappers,
shared depth and element budgets, transactional cursors, unsigned 64-bit
INTEGER and OID support, and payload-redacted diagnostics. It has no ambient
filesystem, network, process, environment, credential, or execution authority.

## Development

```bash
# Run tests
bash BUILD
```
