# Chief of Staff Channel Store (Go)

This package is the Go implementation of the
[D18P portable durable-channel profile](../../../specs/D18P-chief-of-staff-durable-channel-profile.md).
It consumes the repository's single shared fixture corpus at
`code/fixtures/chief-of-staff-channel/v1/manifest.json`.

It provides exact `D18C`, `D18H`, `D18S`, and `D18A` codecs; deterministic
`chief-channels` keys; an injected atomic storage interface; reserve-before-
encrypt recovery; ordered paging; independent receiver cursors; irreversible
destruction; and structurally separate `DurableOriginator` and
`DurableReceiver` APIs.

The package delegates `D18M` encryption and verification to the Go
`chief-of-staff-channel-crypto` package. Backend durability, clocks, UUID
generation, entropy, and key custody are injected. `D18G` grant creation,
opening, and rotation remain owned by issue #141: D18P persists opaque grant
bytes only after enforcing membership, then invokes the receiver key provider
only after the exact grant record is found.

## Development

```bash
go test ./... -v -cover
go vet ./...
```

The shared fixture's private keys and channel master keys are public test-only
material. Errors, records, and APIs must never log or serialize plaintext,
channel master keys, private keys, or opened grant contents.
