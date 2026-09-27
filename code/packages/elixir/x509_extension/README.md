# x509-extension (Elixir)

`CodingAdventures.X509Extension` validates the generic RFC 5280 Extension
sequence above the repository-owned typed DER ASN.1 package. It shares the
caller's depth, element, framing, and OID budgets; rejects encoded default
`critical = FALSE`; keeps extension bytes opaque; and reports stable local
offsets without including payload bytes.

Extension and error values are authenticated opaque handles backed by private
processes. Closure inspection reveals only a type marker and process identifier,
not extension bytes or error state.

The package does not interpret extension payloads, validate certificates or
paths, verify signatures, choose algorithms, access trust stores, decode PEM,
or use filesystem, network, process, environment, clock, entropy, credential,
key, certificate, or console authority.

## Dependency

- der-asn1

## Development

```bash
mix deps.get --quiet
mix format --check-formatted "lib/**/*.{ex,exs}" "test/**/*.{ex,exs}" mix.exs
MIX_ENV=test mix compile --warnings-as-errors
mix test --cover
```

The test suite executes all 48 language-neutral cases and enforces at least 95
percent production line coverage.
