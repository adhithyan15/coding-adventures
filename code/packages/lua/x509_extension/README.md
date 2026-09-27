# x509-extension (Lua)

`coding_adventures.x509_extension` validates the generic RFC 5280 Extension
sequence above the repository-owned typed DER ASN.1 package. It shares the
caller's depth, element, framing, and OID budgets; rejects encoded default
`critical = FALSE`; keeps extension bytes opaque; and reports stable local
offsets without including payload bytes.

Extension and error values keep trusted state in private weak-key registries.
Their metatables are sealed, their visible tables are empty, and no public
constructor can forge either value.

The package does not interpret extension payloads, validate certificates or
paths, verify signatures, choose algorithms, access trust stores, decode PEM,
or use filesystem, network, process, environment, clock, entropy, credential,
key, certificate, or console authority.

## Dependency

- der-asn1

## Development

```bash
bash BUILD
```

The test suite executes all 48 language-neutral cases and enforces at least 95
percent production line coverage.
