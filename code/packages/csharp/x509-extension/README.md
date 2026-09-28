# x509-extension (C#)

`x509-extension` decodes the generic RFC 5280 `Extension` structure over the
adjacent C# `der-asn1` package. It preserves shared depth and element budgets,
uses Extension-local offsets, rejects an explicitly encoded default `critical`
value, and keeps the extension value opaque.

The package consumes all 48 cases in the language-neutral
`x509-extension-v1` profile. Validated values have no public constructor,
their byte payloads are immutable snapshots, and failures are stable and
payload-free. It does not interpret extension-specific schemas, perform
certificate policy or path validation, access trust stores, or use the host.
