# Chief of Staff Channel Store (TypeScript)

This package is the TypeScript implementation of the
[D18P portable durable-channel profile](../../../specs/D18P-chief-of-staff-durable-channel-profile.md).
It consumes the repository's single shared fixture corpus at
`code/fixtures/chief-of-staff-channel/v1/manifest.json`.

It provides:

- canonical, bounded `D18C`, `D18H`, `D18S`, and `D18A` codecs
- the exact `chief-channels` keys and content types
- an injected atomic storage interface plus a deterministic in-memory backend
- reserve-before-encrypt append, retry/recovery, permanent abandonment gaps,
  ordered paging, random access, and independent monotonic receiver cursors
- immutable definition creation and irreversible destruction
- separate `DurableOriginator` and `DurableReceiver` APIs, so a receiver has no
  publish method and an originator has no receive method
- the closed 30-code portable failure roster

The package delegates `D18M` encryption and verification to the TypeScript
`chief-of-staff-channel-crypto` implementation. Backend durability, clocks,
UUID generation, entropy, and key custody are injected. `D18G` grant creation,
opening, and rotation remain owned by issue #141: D18P persists opaque grant
bytes only after enforcing membership, then calls a receiver key provider only
after the exact grant record is found.

## Validation

```sh
npm ci
npx vitest run --coverage
npx tsc --noEmit
```

The conformance test reproduces every shared definition, state, cursor, key,
positive transition, negative transition, oversize recipe, and stable error
roster entry. The deterministic recovery scenario also reproduces the Rust
generator's first complete `D18M` record byte-for-byte.

## Security boundary

Fixture private keys and channel master keys are public test-only material.
Errors, records, and APIs must never log or serialize plaintext, channel master
keys, private keys, or opened grant contents.
