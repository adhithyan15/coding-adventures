# Chief of Staff Channel Crypto (Go)

This package implements the portable D18F immutable encrypted-message profile
for Go Chief of Staff agents. It consumes the shared fixture corpus in
`code/fixtures/chief-of-staff-message/v1` and produces the same authenticated
header, `D18M` version 1 record, canonical JSON, ciphertext, tag, and Ed25519
signature as the production Rust implementation.

The implementation uses repository-owned SHA-256, Ed25519, and
XChaCha20-Poly1305 primitives. It never falls back to D19 `ACTM`, plaintext, a
host crypto API, or a JSON-only envelope.

```go
message, err := channelcrypto.MessageCreate(fields, plaintext, signingSecretKey, channelMasterKey)
durableRecord, err := channelcrypto.MessageSerialize(message)
recovered, err := channelcrypto.MessageVerify(message, originatorPublicKey, channelMasterKey)
```

All byte inputs are copied. `D18Message` keeps its fields unexported and returns
fresh byte slices from accessors. Callers can inject UUID-v7 and monotonic-clock
sources through `MessageCreateWithSources` for deterministic tests and
host-specific clocks.

Errors are `ProfileError` values with the stable D18F `Code` taxonomy.
Verification returns plaintext only after field validation, epoch resolution,
signature verification, AEAD authentication, and plaintext-hash comparison.

## Development

```bash
go test ./... -v -cover
go vet ./...
```

The fixture contains deterministic private keys and channel keys for tests
only. Never use them in production.
