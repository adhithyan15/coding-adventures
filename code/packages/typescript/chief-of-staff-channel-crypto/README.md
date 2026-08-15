# Chief of Staff Channel Crypto (TypeScript)

This package implements the portable D18F immutable encrypted-message profile
and the D18Q channel-key grant profile for TypeScript and Deno-based Chief of
Staff agents. Its D18F implementation consumes the shared fixture corpus in
`code/fixtures/chief-of-staff-message/v1` and produces the same authenticated
header, `D18M` version 1 record, canonical JSON, ciphertext, tag, and Ed25519
signature as the production Rust implementation.

The implementation uses repository-owned SHA-256, Ed25519, and
XChaCha20-Poly1305 primitives. It never falls back to D19 `ACTM`, plaintext, a
host crypto API, or a JSON-only envelope.

```typescript
import {
  messageCreate,
  messageSerialize,
  messageVerify,
} from "@coding-adventures/chief-of-staff-channel-crypto";

const message = messageCreate(fields, plaintext, signingSecretKey, channelMasterKey);
const durableRecord = messageSerialize(message);
const recovered = messageVerify(message, originatorPublicKey, channelMasterKey);
```

All byte inputs are copied. `D18Message` uses runtime-private fields, returns
fresh byte arrays from accessors, and is frozen after construction. Callers can
also inject UUID-v7 and monotonic-clock sources through
`messageCreateWithSources` for deterministic tests and host-specific clocks.

Errors are `MessageProfileError` values with the stable D18F `code` taxonomy.
Verification returns plaintext only after field validation, epoch resolution,
signature verification, AEAD authentication, and plaintext-hash comparison.

## D18Q channel-key grants

The D18Q API consumes
`code/fixtures/chief-of-staff-channel-key-grant/v1/manifest.json` directly and
reproduces the production Rust `D18G` version 1 record. It uses the
repository-owned X25519, HKDF-SHA256, XChaCha20-Poly1305, and Ed25519 packages;
host crypto is used only as the production CSPRNG boundary.

```typescript
import {
  KeyGrantFields,
  ReceiverEpochKeys,
  ReceiverKeyPair,
  grantDeserialize,
  grantSerialize,
  sealChannelKey,
} from "@coding-adventures/chief-of-staff-channel-crypto";

const receiver = ReceiverKeyPair.generate();
const fields = new KeyGrantFields(originatorId, receiverId, channelId, 0n);
const grant = sealChannelKey(fields, cmk, receiver.publicKey, signingKey);
const durableRecord = grantSerialize(grant);

const receiverState = new ReceiverEpochKeys(
  originatorId, receiverId, channelId, receiver, signingKey.publicKey,
);
receiverState.installGrant(grantDeserialize(durableRecord));
const epochZeroCmk = receiverState.key(0n);
```

Structural decoding is deliberately separate from trust. Opening validates
the expected originator, receiver, and channel before signature verification,
X25519 agreement, derivation, and AEAD authentication. Receiver state retains
historic epoch keys, accepts byte-identical retry, rejects conflicts and
decreasing epochs, and mutates only after a higher grant opens successfully.
`planRotation` returns a pure receiver-sorted plan; crash-safe durable epoch
activation remains the separate D18P integration tracked by #11734.

Secret containers provide explicit `destroy()` methods and controlled paths
overwrite owned mutable buffers. `secretErasureCapability()` reports
`best_effort`, because JavaScript garbage collection and untracked runtime
copies prevent a physical-memory guarantee.

## Development

```bash
npm install
npm test
npm run build
```

The fixture contains deterministic private keys and channel keys for tests
only. Never use them in production.
