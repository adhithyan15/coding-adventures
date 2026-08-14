# Chief of Staff Channel Store (Ruby)

This package implements the
[D18P portable durable-channel profile](../../../specs/D18P-chief-of-staff-durable-channel-profile.md)
for Ruby and consumes the single shared fixture at
`code/fixtures/chief-of-staff-channel/v1/manifest.json`.

It provides exact `D18C`, `D18H`, `D18S`, and `D18A` codecs, deterministic
storage keys, an injected atomic CAS backend, reserve-before-encrypt recovery,
ordered paging, independent receiver cursors, irreversible destruction, and
separate durable originator and receiver APIs. `D18M` cryptography is delegated
to the Ruby channel-crypto package. D18P stores D18G grants as opaque bytes and
keeps grant creation, opening, and rotation behind the issue #141 key-provider
boundary.

Run the package checks with `bundle exec rake test`.

Fixture secrets are public test-only material. Errors, records, and APIs must
never log or serialize plaintext, channel master keys, private keys, or opened
grant contents.
