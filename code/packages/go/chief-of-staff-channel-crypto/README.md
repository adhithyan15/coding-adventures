# Chief of Staff Channel Crypto (Go)

This package implements the portable D18F immutable encrypted-message profile
and D18Q channel-key grant profile for Go Chief of Staff agents. It consumes
the shared fixture corpora in `code/fixtures/chief-of-staff-message/v1` and
`code/fixtures/chief-of-staff-channel-key-grant/v1`, producing the same
authenticated `D18M` messages and sealed `D18G` grants as the production Rust
implementation.

The implementation uses repository-owned SHA-256, HKDF, X25519, Ed25519, and
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

## Channel-key grants

D18Q wraps one channel master key for one receiver. Public grant fields remain
unexported and every byte accessor returns a defensive copy. Managed CMK,
receiver-private-key, and signing-key values expose explicit `Destroy` methods;
receiver epoch installation is atomic and monotonic.

```go
cmk, err := channelcrypto.GenerateChannelMasterKey()
grant, err := channelcrypto.SealChannelKey(
    fields, cmk, receiverPublicKey, signingKey,
)
receiverEpochs, err := channelcrypto.NewReceiverEpochKeys(
    fields.OriginatorID(), fields.ReceiverID(), fields.ChannelID(),
    receiverKeyPair, originatorPublicKey,
)
outcome, err := receiverEpochs.InstallGrant(grant)
epochKey, err := receiverEpochs.Key(fields.KeyEpoch())
```

### Verifying a grant you cannot open

Opening a grant needs the receiver's private key, because unwrapping performs an
X25519 agreement. An originator holds no receiver secrets, so it cannot open the
grants it seals — yet D18T requires it to verify the originator signature on
every receiver's grant before a rotation candidate may be offered to key
custody. `VerifyGrantSignature` is that weaker, receiver-key-free check:

```go
err := channelcrypto.VerifyGrantSignature(
    grant, expectedOriginatorID, expectedReceiverID, channelID, originatorPublicKey,
)
```

It proves the originator signed this exact `(originator, receiver, channel,
epoch, ephemeral key, nonce, wrapped CMK)` tuple. It proves nothing about
whether the wrapped CMK decrypts — only unwrapping can establish that, so a
caller verifying grants it cannot open must not treat success as proof the
receiver will be able to use the grant. Both entry points share one verification
path, so they always agree on binding order and stable error codes.

`PlanRotation` creates a complete receiver-sorted prospective plan and emits no
grant for a revoked receiver. D18Q failures are `KeyGrantProfileError` values
with stable codes. `GrantSecretErasureCapability()` honestly reports
`best_effort`: owned arrays and slices are cleared on controlled destruction,
but Go value copies, garbage collection, and repository primitive intermediates
prevent a physical-memory guarantee.

## Development

```bash
go test ./... -v -cover
go vet ./...
```

The fixture contains deterministic private keys and channel keys for tests
only. Never use them in production.
