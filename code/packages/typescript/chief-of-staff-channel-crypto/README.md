# Chief of Staff Channel Crypto (TypeScript)

This package implements the portable D18F immutable encrypted-message profile
for TypeScript and Deno-based Chief of Staff agents. It consumes the shared
fixture corpus in `code/fixtures/chief-of-staff-message/v1` and produces the
same authenticated header, `D18M` version 1 record, canonical JSON, ciphertext,
tag, and Ed25519 signature as the production Rust implementation.

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

## Development

```bash
npm install
npm test
npm run build
```

The fixture contains deterministic private keys and channel keys for tests
only. Never use them in production.
