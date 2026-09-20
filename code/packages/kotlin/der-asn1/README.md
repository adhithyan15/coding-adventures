# DER ASN.1 for Kotlin

Bounded typed ASN.1 DER value decoding over the language-neutral
`der-asn1-v1` behavior contract. The package consumes all 109 portable cases,
including all 46 referenced DER-TLV framing cases.

The typed layer adds canonical primitive decoders, exact constructed wrappers,
shared depth and element budgets, transactional cursors, unsigned 64-bit
INTEGER and OID support, and payload-redacted diagnostics. It has no ambient
filesystem, network, process, environment, credential, or execution authority.
Validated wrappers use private JVM constructors, defensive byte snapshots, and
runtime-unmodifiable OID arc lists so Java callers cannot forge or mutate them.

## Development

```bash
bash BUILD
```
