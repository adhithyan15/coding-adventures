# der-asn1 (Elixir)

`CodingAdventures.DerAsn1` is a bounded, payload-blind typed ASN.1 DER layer
above the repository-owned `CodingAdventures.DerTlv` framing package. It
validates canonical BOOLEAN, INTEGER, BIT STRING, OCTET STRING, IA5String,
NULL, OBJECT IDENTIFIER, SEQUENCE, SET, and context-specific implicit and
explicit values.

Decoder, element, cursor, and typed-value payloads live in private immutable
handle processes; closure introspection reveals only the value kind and process
identifier. Owner tokens bind every descendant and cursor to its originating
decoder. A restricted private decoder-state process owns canonical limits, the
total-element budget, and cursor progress across every alias; failed parsing
neither advances a cursor nor consumes work.

This package has an empty capability manifest. It does not parse X.509 schema,
validate certificate paths or signatures, access trust stores, perform TLS,
decode PEM, or use filesystem, network, process, clock, entropy, environment,
credential, or console authority.

## Development

```bash
mix deps.get --quiet
mix format --check-formatted
MIX_ENV=test mix compile --warnings-as-errors
mix test --cover
```

The neutral fixture suite executes all 122 DER ASN.1 cases and all 46 referenced
DER TLV framing cases.
